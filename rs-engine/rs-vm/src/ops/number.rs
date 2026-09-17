use crate::engine::engine;
use crate::engine::{ScriptEngine, engine_mut};
use crate::register::OpsRegistry;
use crate::util::RUNEDAY_EPOCH_DAYS;
use crate::{handlers, none};
use rs_pack::cache::script::*;
use rs_util::bits::{clearbit_range, setbit_range, setbit_range_toint};

/// Registers arithmetic, bitwise, trigonometric, and random number opcodes.
///
/// All operations use wrapping semantics for overflow safety and operate on
/// the script's integer stack.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Arithmetic:** `ADD`, `SUB`, `MULTIPLY`, `DIVIDE`, `MODULO`, `ABS`,
///   `ADDPERCENT`, `SCALE`, `INTERPOLATE`
/// - **Power / roots:** `POW`, `INVPOW`
/// - **Bitwise:** `AND`, `OR`, `SETBIT`, `CLEARBIT`, `TESTBIT`, `TOGGLEBIT`,
///   `BITCOUNT`, `SETBIT_RANGE`, `CLEARBIT_RANGE`, `GETBIT_RANGE`, `SETBIT_RANGE_TOINT`
/// - **Comparison:** `MIN`, `MAX`
/// - **Trigonometry:** `SIN_DEG`, `COS_DEG`, `ATAN2_DEG`
/// - **Random:** `RANDOM`, `RANDOMINC`
/// - **Date** `DATE_MINUTES`, `DATE_RUNEDAY`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::number::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 4600
        none!(m, ADD => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.wrapping_add(b));
        });

        // 4601
        none!(m, SUB => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.wrapping_sub(b));
        });

        // 4602
        none!(m, MULTIPLY => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.wrapping_mul(b));
        });

        // 4603
        none!(m, DIVIDE => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.wrapping_div(b));
        });

        // 4604
        none!(m, RANDOM => |s| {
            let a = s.pop_int();
            s.push_int((engine_mut::<E>().random().next_double() * a as f64) as i32);
        });

        // 4605
        none!(m, RANDOMINC => |s| {
            let a = s.pop_int();
            s.push_int((engine_mut::<E>().random().next_double() * (a + 1) as f64) as i32);
        });

        // 4606
        none!(m, INTERPOLATE => |s| {
            let e = s.pop_int();
            let d = s.pop_int();
            let c = s.pop_int();
            let b = s.pop_int();
            let a = s.pop_int();
            let floor: f64 = (b.wrapping_sub(a) as f64 / d.wrapping_sub(c) as f64).floor();
            s.push_int(((floor * e.wrapping_sub(c) as f64) + a as f64) as i32);
        });

        // 4607
        none!(m, ADDPERCENT => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(
                a.wrapping_mul(b)
                    .wrapping_div(100)
                    .wrapping_add(a),
            );
        });

        // 4608
        none!(m, SETBIT => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a | (1i32.wrapping_shl(b as u32)));
        });

        // 4609
        none!(m, CLEARBIT => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a & !1i32.wrapping_shl(b as u32));
        });

        // 4610
        none!(m, TESTBIT => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(((a & (1i32.wrapping_shl(b as u32))) != 0) as i32);
        });

        // 4611
        none!(m, MODULO => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.wrapping_rem(b));
        });

        // 4612
        none!(m, POW => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.wrapping_pow(b as u32));
        });

        // 4613
        none!(m, INVPOW => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            if a == 0 || b == 0 {
                s.push_int(0);
            } else {
                match b {
                    1 => s.push_int(a),
                    2 => s.push_int((a as f64).sqrt() as i32),
                    3 => s.push_int((a as f64).cbrt() as i32),
                    4 => s.push_int((a as f64).sqrt().sqrt() as i32),
                    _ => s.push_int(a.pow((1.0 / b as f64) as u32)),
                }
            }
        });

        // 4614
        none!(m, AND => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a & b);
        });

        // 4615
        none!(m, OR => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a | b);
        });

        // 4616
        none!(m, MIN => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.min(b));
        });

        // 4617
        none!(m, MAX => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.max(b));
        });

        // 4618
        none!(m, SCALE => |s| {
            let c = s.pop_int();
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a.wrapping_mul(c).wrapping_div(b));
        });

        // 4619
        none!(m, BITCOUNT => |s| {
            let a = s.pop_int();
            s.push_int(a.count_ones() as i32);
        });

        // 4620
        none!(m, TOGGLEBIT => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(a ^ (1 << b));
        });

        // 4621
        none!(m, SETBIT_RANGE => |s| {
            let c = s.pop_int();
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(setbit_range(a, b, c));
        });

        // 4622
        none!(m, CLEARBIT_RANGE => |s| {
            let c = s.pop_int();
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(clearbit_range(c, b, a));
        });

        // 4623
        none!(m, GETBIT_RANGE => |s| {
            let c = s.pop_int();
            let b = s.pop_int();
            let a = s.pop_int();
            let r: i32 = 31i32.wrapping_sub(c);
            s.push_int(((a.wrapping_shl(r as u32) as u32) >> (b.wrapping_add(r) as u32)) as i32);
        });

        // 4624
        none!(m, SETBIT_RANGE_TOINT => |s| {
            let d = s.pop_int();
            let c = s.pop_int();
            let b = s.pop_int();
            let a = s.pop_int();
            s.push_int(setbit_range_toint(a, b, c, d));
        });

        // 4625
        none!(m, SIN_DEG => |s| {
            let a = s.pop_int();
            let rad = (a as f64) * std::f64::consts::PI / (180.0 * 65536.0);
            s.push_int((rad.sin() * 65536.0) as i32);
        });

        // 4626
        none!(m, COS_DEG => |s| {
            let a = s.pop_int();
            let rad = (a as f64) * std::f64::consts::PI / (180.0 * 65536.0);
            s.push_int((rad.cos() * 65536.0) as i32);
        });

        // 4627
        none!(m, ATAN2_DEG => |s| {
            let b = s.pop_int();
            let a = s.pop_int();
            let rad = (a as f64).atan2(b as f64);
            s.push_int((rad * 180.0 * 65536.0 / std::f64::consts::PI) as i32);
        });

        // 4628
        none!(m, ABS => |s| {
            let a = s.pop_int();
            s.push_int(a.abs());
        });

        // 4629
        none!(m, DATE_MINUTES => |s| {
            s.push_int((engine::<E>().date_millis() / 60_000) as i32);
        });

        // 4630
        none!(m, DATE_RUNEDAY => |s| {
            s.push_int((engine::<E>().date_millis() / 86_400_000) as i32 - RUNEDAY_EPOCH_DAYS);
        });
    }
}
