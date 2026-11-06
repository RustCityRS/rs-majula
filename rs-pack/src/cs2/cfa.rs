//! Control flow analysis — converts flat IR instructions into structured
//! constructs (if/else, while, switch).
//!
//! Uses a simple forward-scan algorithm that detects common patterns without
//! a full dominator-tree analysis. Sufficient for the majority of CS2 scripts.

use super::opcodes;
use crate::cs2::ir::*;
use std::collections::BTreeMap;
use std::mem::take;

/// A structured control-flow construct.
#[derive(Debug, Clone)]
pub enum Construct {
    /// A sequence of flat IR instructions, followed by an optional successor.
    Seq(Vec<Instruction>, Option<Box<Construct>>),
    /// An if/else construct.
    If {
        condition: Expr,
        then_body: Box<Construct>,
        else_body: Option<Box<Construct>>,
        next: Option<Box<Construct>>,
    },
    /// A while loop.
    While {
        condition: Expr,
        body: Box<Construct>,
        next: Option<Box<Construct>>,
    },
    /// A switch statement.
    Switch {
        expr: Expr,
        cases: Vec<(Vec<i32>, Box<Construct>)>,
        default: Option<Box<Construct>>,
        next: Option<Box<Construct>>,
    },
}

/// A basic block: a label (if any) and a list of instructions.
struct BasicBlock {
    label: Option<u32>,
    instructions: Vec<Instruction>,
}

/// Reconstruct structured control flow from a flat `Function`.
pub fn reconstruct(function: &Function) -> Construct {
    // Step 1: split into basic blocks
    let blocks = split_into_blocks(&function.instructions);

    // Step 2: recursively structure the blocks
    if blocks.is_empty() {
        return Construct::Seq(Vec::new(), None);
    }

    structure_blocks(&blocks, 0)
}

/// Split instructions into basic blocks at labels, gotos, branches, switches,
/// and returns.
fn split_into_blocks(instructions: &[Instruction]) -> Vec<BasicBlock> {
    let mut blocks: Vec<BasicBlock> = Vec::new();
    let mut current_label: Option<u32> = None;
    let mut current_instrs: Vec<Instruction> = Vec::new();

    for instr in instructions {
        match instr {
            Instruction::Label(lbl) => {
                // Flush current block
                if !current_instrs.is_empty() || current_label.is_some() {
                    blocks.push(BasicBlock {
                        label: current_label,
                        instructions: take(&mut current_instrs),
                    });
                }
                current_label = Some(*lbl);
            }
            Instruction::Goto(_)
            | Instruction::Return { .. }
            | Instruction::Branch { .. }
            | Instruction::Switch { .. } => {
                current_instrs.push(instr.clone());
                blocks.push(BasicBlock {
                    label: current_label.take(),
                    instructions: take(&mut current_instrs),
                });
            }
            _ => {
                current_instrs.push(instr.clone());
            }
        }
    }

    // Flush trailing block
    if !current_instrs.is_empty() || current_label.is_some() {
        blocks.push(BasicBlock {
            label: current_label,
            instructions: current_instrs,
        });
    }

    blocks
}

/// Find the block index that contains the given label.
fn find_block_with_label(blocks: &[BasicBlock], label: u32) -> Option<usize> {
    blocks.iter().position(|b| b.label == Some(label))
}

/// Recursively structure blocks starting at `start_idx`.
fn structure_blocks(blocks: &[BasicBlock], start_idx: usize) -> Construct {
    if start_idx >= blocks.len() {
        return Construct::Seq(Vec::new(), None);
    }

    let block = &blocks[start_idx];
    let instrs = &block.instructions;

    if instrs.is_empty() {
        // Empty block (just a label) — continue to next
        let next = if start_idx + 1 < blocks.len() {
            Some(Box::new(structure_blocks(blocks, start_idx + 1)))
        } else {
            None
        };
        return Construct::Seq(Vec::new(), next);
    }

    let last = &instrs[instrs.len() - 1];
    let preamble: Vec<Instruction> = instrs[..instrs.len() - 1].to_vec();

    match last {
        // Unconditional goto — emit preamble then continue at target
        Instruction::Goto(target) => {
            let target_idx = find_block_with_label(blocks, *target);

            // Check for while loop: goto jumps backward
            if let Some(t_idx) = target_idx {
                if t_idx <= start_idx {
                    // This is a loop back-edge. Look at the target block for
                    // the loop condition.
                    let target_block = &blocks[t_idx];
                    let cond_and_exit = extract_branch_condition(target_block);

                    if let Some((condition, exit_target)) = cond_and_exit {
                        let exit_idx = find_block_with_label(blocks, exit_target);
                        // The loop body is blocks from t_idx+1 .. start_idx (inclusive)
                        let body = structure_range(blocks, t_idx + 1, start_idx + 1);
                        let next_idx = exit_idx.unwrap_or(start_idx + 1);
                        let next = if next_idx < blocks.len() {
                            Some(Box::new(structure_blocks(blocks, next_idx)))
                        } else {
                            None
                        };
                        // We wrap everything in a Seq leading to the While
                        return wrap_seq(
                            preamble,
                            Construct::While {
                                condition,
                                body: Box::new(body),
                                next,
                            },
                        );
                    }
                }
            }

            // Forward goto: jump directly to the target block, skipping
            // intervening blocks (they're either dead code or already
            // handled by the switch/if default body detection).
            if let Some(t_idx) = target_idx {
                let next = if t_idx < blocks.len() {
                    Some(Box::new(structure_blocks(blocks, t_idx)))
                } else {
                    None
                };
                Construct::Seq(preamble, next)
            } else {
                // External target — drop the goto
                Construct::Seq(preamble, None)
            }
        }

        // Conditional branch — try to detect if/else
        Instruction::Branch { condition, target } => {
            let target_idx = find_block_with_label(blocks, *target);

            if let Some(t_idx) = target_idx {
                if t_idx > start_idx + 1 {
                    // Inverted pattern: the fall-through from the branch
                    // is a single bare Goto (jumping to else/merge), and
                    // the branch target is the then body.  The branch is
                    // taken when the condition is TRUE, so we don't negate.
                    let inverted = t_idx == start_idx + 2
                        && blocks[start_idx + 1].label.is_none()
                        && blocks[start_idx + 1].instructions.len() == 1
                        && matches!(
                            blocks[start_idx + 1].instructions.first(),
                            Some(Instruction::Goto(_))
                        );

                    if inverted {
                        let else_target = extract_goto_target(&blocks[start_idx + 1]).unwrap();
                        let else_idx = find_block_with_label(blocks, else_target);

                        // Merge = then-body's trailing goto (indicates
                        // an else branch exists after the then body).
                        let e_idx = else_idx.unwrap_or(blocks.len());
                        let merge = if e_idx > t_idx {
                            extract_goto_target(&blocks[e_idx - 1])
                        } else {
                            None
                        };
                        let m_idx = merge
                            .and_then(|t| find_block_with_label(blocks, t))
                            .unwrap_or(e_idx);

                        let then_body = structure_range_strip_merge(blocks, t_idx, e_idx, merge);

                        let (else_body, next) = if merge.is_some() && e_idx < m_idx {
                            let eb = Some(Box::new(structure_range_strip_merge(
                                blocks, e_idx, m_idx, merge,
                            )));
                            let n = if m_idx < blocks.len() {
                                Some(Box::new(structure_blocks(blocks, m_idx)))
                            } else {
                                None
                            };
                            (eb, n)
                        } else {
                            let n = if e_idx < blocks.len() {
                                Some(Box::new(structure_blocks(blocks, e_idx)))
                            } else {
                                None
                            };
                            (None, n)
                        };

                        return wrap_seq(
                            preamble,
                            Construct::If {
                                condition: condition.clone(),
                                then_body: Box::new(then_body),
                                else_body,
                                next,
                            },
                        );
                    }

                    // Normal pattern: then body between branch and target,
                    // branch skips when condition is true (we negate).
                    let pre_target = &blocks[t_idx - 1];
                    let else_goto = extract_goto_target(pre_target);

                    if let Some(else_end) = else_goto {
                        let else_end_idx = find_block_with_label(blocks, else_end);
                        let then_body = structure_range_strip_merge(
                            blocks,
                            start_idx + 1,
                            t_idx,
                            Some(else_end),
                        );
                        let else_end_i = else_end_idx.unwrap_or(blocks.len());
                        let else_body = if t_idx < else_end_i {
                            Some(Box::new(structure_range_strip_merge(
                                blocks,
                                t_idx,
                                else_end_i,
                                Some(else_end),
                            )))
                        } else {
                            None
                        };
                        let next = if else_end_i < blocks.len() {
                            Some(Box::new(structure_blocks(blocks, else_end_i)))
                        } else {
                            None
                        };
                        return wrap_seq(
                            preamble,
                            Construct::If {
                                condition: negate_condition(condition.clone()),
                                then_body: Box::new(then_body),
                                else_body,
                                next,
                            },
                        );
                    }

                    // No else branch
                    let then_body = structure_range(blocks, start_idx + 1, t_idx);
                    let next = if t_idx < blocks.len() {
                        Some(Box::new(structure_blocks(blocks, t_idx)))
                    } else {
                        None
                    };
                    return wrap_seq(
                        preamble,
                        Construct::If {
                            condition: negate_condition(condition.clone()),
                            then_body: Box::new(then_body),
                            else_body: None,
                            next,
                        },
                    );
                }
            }

            // Branch target not found or at next block.
            // Wrap remaining blocks in if(!cond){...} to eliminate the
            // goto — the branch is an early exit from this scope.
            if start_idx + 1 < blocks.len() {
                let body = structure_blocks(blocks, start_idx + 1);
                if is_empty_construct(&body) {
                    // Nothing meaningful after the branch — drop it
                    Construct::Seq(preamble, None)
                } else {
                    wrap_seq(
                        preamble,
                        Construct::If {
                            condition: negate_condition(condition.clone()),
                            then_body: Box::new(body),
                            else_body: None,
                            next: None,
                        },
                    )
                }
            } else {
                // Last instruction in scope — drop the goto
                Construct::Seq(preamble, None)
            }
        }

        // Switch
        Instruction::Switch { expr, cases } => {
            // Group cases by target label, track block indices
            let mut target_cases: BTreeMap<u32, Vec<i32>> = BTreeMap::new();
            let mut max_case_block = start_idx;
            for &(key, target) in cases {
                target_cases.entry(target).or_default().push(key);
                if let Some(idx) = find_block_with_label(blocks, target) {
                    if idx > max_case_block {
                        max_case_block = idx;
                    }
                }
            }

            // Block start_idx+1 is always the default fallthrough goto.
            // Skip it when scanning for the merge point so it doesn't get
            // confused with the actual case-merge gotos.
            let default_goto_target = blocks.get(start_idx + 1).and_then(extract_goto_target);
            let default_body_idx =
                default_goto_target.and_then(|t| find_block_with_label(blocks, t));

            let mut end_idx = blocks.len();
            let mut external_merge: Option<u32> = None;
            for i in (start_idx + 2)..blocks.len() {
                if let Some(Instruction::Goto(goto_target)) = blocks[i].instructions.last() {
                    if let Some(g_idx) = find_block_with_label(blocks, *goto_target) {
                        if g_idx > max_case_block {
                            end_idx = g_idx;
                            break;
                        }
                    } else {
                        // Target is outside sub_blocks — record as potential
                        // external merge label (common in nested switches).
                        external_merge = Some(*goto_target);
                        break;
                    }
                }
            }

            let merge_label = if end_idx < blocks.len() {
                blocks[end_idx].label
            } else {
                external_merge
            };

            // Sort case targets by block position
            let mut sorted_targets: Vec<(u32, usize)> = target_cases
                .keys()
                .filter_map(|&t| find_block_with_label(blocks, t).map(|idx| (t, idx)))
                .collect();
            sorted_targets.sort_by_key(|&(_, idx)| idx);

            // If the default body is between case blocks and the merge
            // point (and not the merge point itself), the last case's
            // boundary stops there.
            let default_body_between_cases = default_body_idx
                .filter(|&d| d > max_case_block && d < end_idx)
                .filter(|&d| blocks[d].label != merge_label);
            let last_case_end = default_body_between_cases.unwrap_or(end_idx);

            let mut structured_cases = Vec::new();
            for (i, &(target_label, t_idx)) in sorted_targets.iter().enumerate() {
                let keys = target_cases.get(&target_label).unwrap().clone();
                let mut next_boundary = sorted_targets
                    .get(i + 1)
                    .map(|&(_, idx)| idx)
                    .unwrap_or(last_case_end);
                // If the default body falls between this case and the next,
                // stop the case body before the default body.
                if let Some(d_idx) = default_body_idx {
                    if d_idx > t_idx && d_idx < next_boundary {
                        next_boundary = d_idx;
                    }
                }
                let body = structure_range_strip_merge(blocks, t_idx, next_boundary, merge_label);
                structured_cases.push((keys, Box::new(body)));
            }

            // Default body: follow the default fallthrough goto if it
            // points to a body between cases and merge. Otherwise use
            // blocks between switch and first case target.
            let first_case_idx = sorted_targets
                .first()
                .map(|&(_, idx)| idx)
                .unwrap_or(end_idx);
            let default = if let Some(d_idx) = default_body_between_cases {
                Some(Box::new(structure_range_strip_merge(
                    blocks,
                    d_idx,
                    end_idx,
                    merge_label,
                )))
            } else {
                let default_start = start_idx + 1;
                // A bare goto to a label not in the sub_blocks means
                // "no default body — jump straight to the (external)
                // merge point". Don't emit it as a default case.
                let is_external_goto = blocks
                    .get(default_start)
                    .filter(|b| b.instructions.len() == 1)
                    .and_then(extract_goto_target)
                    .is_some_and(|t| find_block_with_label(blocks, t).is_none());
                if !is_external_goto && default_start < first_case_idx {
                    Some(Box::new(structure_range_strip_merge(
                        blocks,
                        default_start,
                        first_case_idx,
                        merge_label,
                    )))
                } else {
                    None
                }
            };

            let next = if end_idx < blocks.len() {
                Some(Box::new(structure_blocks(blocks, end_idx)))
            } else {
                None
            };

            wrap_seq(
                preamble,
                Construct::Switch {
                    expr: expr.clone(),
                    cases: structured_cases,
                    default,
                    next,
                },
            )
        }

        // Return — only continue to next block if it has a label
        // (branch target), otherwise it's unreachable dead code.
        Instruction::Return { .. } => {
            let mut all = preamble;
            all.push(last.clone());
            let next = if start_idx + 1 < blocks.len() && blocks[start_idx + 1].label.is_some() {
                Some(Box::new(structure_blocks(blocks, start_idx + 1)))
            } else {
                None
            };
            Construct::Seq(all, next)
        }

        // Any other instruction — shouldn't be last but handle gracefully
        _ => {
            let next = if start_idx + 1 < blocks.len() {
                Some(Box::new(structure_blocks(blocks, start_idx + 1)))
            } else {
                None
            };
            Construct::Seq(instrs.to_vec(), next)
        }
    }
}

/// Structure a range of blocks [from..to) into a single Construct.
fn structure_range(blocks: &[BasicBlock], from: usize, to: usize) -> Construct {
    structure_range_strip_merge(blocks, from, to, None)
}

/// Structure a range of blocks, stripping any trailing Goto to the merge label.
fn structure_range_strip_merge(
    blocks: &[BasicBlock],
    from: usize,
    to: usize,
    merge_label: Option<u32>,
) -> Construct {
    if from >= to || from >= blocks.len() {
        return Construct::Seq(Vec::new(), None);
    }
    let end = to.min(blocks.len());
    let mut sub_blocks: Vec<BasicBlock> = blocks[from..end]
        .iter()
        .map(|b| BasicBlock {
            label: b.label,
            instructions: b.instructions.clone(),
        })
        .collect();

    let should_strip_last = sub_blocks
        .last()
        .and_then(|b| match b.instructions.last() {
            Some(Instruction::Goto(t)) => {
                let target = *t;
                let matches_merge = merge_label == Some(target);
                let is_external = find_block_with_label(&sub_blocks, target).is_none();
                if matches_merge || is_external {
                    Some(())
                } else {
                    None
                }
            }
            _ => None,
        })
        .is_some();
    if should_strip_last {
        if let Some(last_block) = sub_blocks.last_mut() {
            last_block.instructions.pop();
        }
    }

    // Remove dead-code goto blocks: a bare Goto following a Return is
    // unreachable and can be stripped regardless of merge label.
    let mut i = 1;
    while i < sub_blocks.len() {
        let prev_returns = matches!(
            sub_blocks[i - 1].instructions.last(),
            Some(Instruction::Return { .. })
        );
        let is_bare_goto = sub_blocks[i].label.is_none()
            && sub_blocks[i].instructions.len() == 1
            && matches!(
                sub_blocks[i].instructions.first(),
                Some(Instruction::Goto(_))
            );
        if prev_returns && is_bare_goto {
            sub_blocks.remove(i);
        } else {
            i += 1;
        }
    }

    if sub_blocks.is_empty() {
        Construct::Seq(Vec::new(), None)
    } else {
        structure_blocks(&sub_blocks, 0)
    }
}

/// If the last instruction of a block is a branch, extract its condition and target.
fn extract_branch_condition(block: &BasicBlock) -> Option<(Expr, u32)> {
    block.instructions.last().and_then(|instr| {
        if let Instruction::Branch { condition, target } = instr {
            Some((condition.clone(), *target))
        } else {
            None
        }
    })
}

/// If the last instruction of a block is a Goto, return its target.
fn extract_goto_target(block: &BasicBlock) -> Option<u32> {
    block.instructions.last().and_then(|instr| {
        if let Instruction::Goto(target) = instr {
            Some(*target)
        } else {
            None
        }
    })
}

/// Negate a branch condition for if-body generation.
///
/// Public negate for use by codegen.
pub fn negate_condition_expr(expr: Expr) -> Expr {
    negate_condition(expr)
}

/// CS2 branches skip the then-body, so the condition must be negated:
///   bytecode: `branch_equals target` → "if NOT equals, run then-body"
fn negate_condition(expr: Expr) -> Expr {
    match expr {
        Expr::Operation { opcode, args } => {
            let negated = match opcode {
                opcodes::BRANCH_EQUALS => opcodes::BRANCH_NOT,
                opcodes::BRANCH_NOT => opcodes::BRANCH_EQUALS,
                opcodes::BRANCH_LESS_THAN => opcodes::BRANCH_GREATER_THAN_OR_EQUALS,
                opcodes::BRANCH_GREATER_THAN => opcodes::BRANCH_LESS_THAN_OR_EQUALS,
                opcodes::BRANCH_LESS_THAN_OR_EQUALS => opcodes::BRANCH_GREATER_THAN,
                opcodes::BRANCH_GREATER_THAN_OR_EQUALS => opcodes::BRANCH_LESS_THAN,
                opcodes::BRANCH_EQUALS_LONG => opcodes::BRANCH_NOT_LONG,
                opcodes::BRANCH_NOT_LONG => opcodes::BRANCH_EQUALS_LONG,
                opcodes::BRANCH_LESS_THAN_LONG => opcodes::BRANCH_GREATER_THAN_OR_EQUALS_LONG,
                opcodes::BRANCH_GREATER_THAN_LONG => opcodes::BRANCH_LESS_THAN_OR_EQUALS_LONG,
                opcodes::BRANCH_LESS_THAN_OR_EQUALS_LONG => opcodes::BRANCH_GREATER_THAN_LONG,
                opcodes::BRANCH_GREATER_THAN_OR_EQUALS_LONG => opcodes::BRANCH_LESS_THAN_LONG,
                _ => return Expr::Operation { opcode, args },
            };
            Expr::Operation {
                opcode: negated,
                args,
            }
        }
        other => other,
    }
}

/// Wrap preamble instructions + a construct into a Seq if preamble is non-empty.
fn wrap_seq(preamble: Vec<Instruction>, construct: Construct) -> Construct {
    if preamble.is_empty() {
        construct
    } else {
        Construct::Seq(preamble, Some(Box::new(construct)))
    }
}

/// Check if a construct is empty (no instructions, no nested constructs).
pub fn is_empty_construct(construct: &Construct) -> bool {
    match construct {
        Construct::Seq(instrs, next) => {
            instrs.is_empty() && next.as_ref().is_none_or(|n| is_empty_construct(n))
        }
        _ => false,
    }
}
