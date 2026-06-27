use crate::types::{IfButtonType, IfComponentType};
use rs_io::Packet;
use std::collections::HashMap;

pub struct IfType {
    pub id: u16,
    pub root_layer: i32,
    pub com_name: Option<Box<str>>,
    pub overlay: bool,
    pub button_type: IfButtonType,
    pub width: u16,
    pub height: u16,
    pub draggable: bool,
    pub operable: bool,
    pub usable: bool,
    pub iop: Option<Box<[Option<Box<str>>]>>,
    pub action_target: u16,
}

impl From<IfTypeRaw> for IfType {
    fn from(raw: IfTypeRaw) -> Self {
        IfType {
            id: raw.id,
            root_layer: raw.root_layer,
            com_name: raw.com_name,
            overlay: raw.overlay,
            button_type: raw.button_type,
            width: raw.width,
            height: raw.height,
            draggable: raw.draggable,
            operable: raw.operable,
            usable: raw.usable,
            iop: raw.iop,
            action_target: raw.action_target,
        }
    }
}

pub struct IfTypeRaw {
    pub id: u16,
    pub root_layer: i32,
    pub com_name: Option<Box<str>>,
    pub overlay: bool,
    pub com_type: IfComponentType,
    pub button_type: IfButtonType,
    pub client_code: u16,
    pub width: u16,
    pub height: u16,
    pub over_layer: i32,
    pub script_comparator: Option<Box<[u8]>>,
    pub script_operand: Option<Box<[u16]>>,
    pub scripts: Option<Box<[Box<[u16]>]>>,
    pub scroll: u16,
    pub hide: bool,
    pub draggable: bool,
    pub operable: bool,
    pub usable: bool,
    #[cfg(since_245_2)]
    pub swappable: bool,
    pub margin_x: u8,
    pub margin_y: u8,
    pub inventory_slot_offset_x: Option<Box<[i16]>>,
    pub inventory_slot_offset_y: Option<Box<[i16]>>,
    pub inventory_slot_graphic: Option<Box<[Option<Box<str>>]>>,
    pub iop: Option<Box<[Option<Box<str>>]>>,
    pub fill: bool,
    pub center: bool,
    pub font: u8,
    pub shadowed: bool,
    pub text: Option<Box<str>>,
    pub active_text: Option<Box<str>>,
    pub colour: i32,
    pub active_colour: i32,
    pub over_colour: i32,
    #[cfg(since_245_2)]
    pub active_over_colour: i32,
    pub graphic: Option<Box<str>>,
    pub active_graphic: Option<Box<str>>,
    pub model: i32,
    pub active_model: i32,
    pub anim: i32,
    pub active_anim: i32,
    pub zoom: u16,
    pub xan: u16,
    pub yan: u16,
    pub action_verb: Option<Box<str>>,
    pub action: Option<Box<str>>,
    pub action_target: u16,
    pub option: Option<Box<str>>,
    pub child_x: Option<Box<[i16]>>,
    pub child_y: Option<Box<[i16]>>,
}

pub struct IfTypeProvider {
    pub names: HashMap<Box<str>, u16>,
    pub types: Vec<Option<Box<IfType>>>,
}

impl IfTypeProvider {
    pub fn from_bytes(dat: &[u8]) -> IfTypeProvider {
        let mut buf = Packet::from(dat.to_vec());
        let count = buf.g2() as usize;

        let mut names = HashMap::new();
        let mut types: Vec<Option<Box<IfType>>> = (0..count).map(|_| None).collect();

        let mut root_layer: i32 = -1;

        while buf.remaining() > 0 {
            let mut id = buf.g2();
            if id == 0xFFFF {
                root_layer = buf.g2() as i32;
                id = buf.g2();
            }

            let mut com = IfTypeRaw {
                id,
                root_layer,
                com_name: None,
                overlay: false,
                com_type: IfComponentType::Layer,
                button_type: IfButtonType::None,
                client_code: 0,
                width: 0,
                height: 0,
                over_layer: -1,
                script_comparator: None,
                script_operand: None,
                scripts: None,
                scroll: 0,
                hide: false,
                draggable: false,
                operable: false,
                usable: false,
                #[cfg(since_245_2)]
                swappable: false,
                margin_x: 0,
                margin_y: 0,
                inventory_slot_offset_x: None,
                inventory_slot_offset_y: None,
                inventory_slot_graphic: None,
                iop: None,
                fill: false,
                center: false,
                font: 0,
                shadowed: false,
                text: None,
                active_text: None,
                colour: 0,
                active_colour: 0,
                over_colour: 0,
                #[cfg(since_245_2)]
                active_over_colour: 0,
                graphic: None,
                active_graphic: None,
                model: -1,
                active_model: -1,
                anim: -1,
                active_anim: -1,
                zoom: 0,
                xan: 0,
                yan: 0,
                action_verb: None,
                action: None,
                action_target: 0,
                option: None,
                child_x: None,
                child_y: None,
            };

            com.com_name = Some(buf.gjstr(10).into_boxed_str());
            com.overlay = buf.g1() == 1;

            com.com_type = IfComponentType::try_from(buf.g1()).unwrap();
            com.button_type = IfButtonType::try_from(buf.g1()).unwrap();
            com.client_code = buf.g2();
            com.width = buf.g2();
            com.height = buf.g2();

            let over_layer = buf.g1() as i32;
            if over_layer == 0 {
                com.over_layer = -1;
            } else {
                com.over_layer = ((over_layer - 1) << 8) + buf.g1() as i32;
            }

            let comparator_count = buf.g1() as usize;
            if comparator_count > 0 {
                let mut comparators = vec![0u8; comparator_count];
                let mut operands = vec![0u16; comparator_count];
                for i in 0..comparator_count {
                    comparators[i] = buf.g1();
                    operands[i] = buf.g2();
                }
                com.script_comparator = Some(comparators.into_boxed_slice());
                com.script_operand = Some(operands.into_boxed_slice());
            }

            let script_count = buf.g1() as usize;
            if script_count > 0 {
                let mut scripts = Vec::with_capacity(script_count);
                for _ in 0..script_count {
                    let opcode_count = buf.g2() as usize;
                    let opcodes: Vec<u16> = (0..opcode_count).map(|_| buf.g2()).collect();
                    scripts.push(opcodes.into_boxed_slice());
                }
                com.scripts = Some(scripts.into_boxed_slice());
            }

            match com.com_type {
                IfComponentType::Layer => {
                    com.scroll = buf.g2();
                    com.hide = buf.g1() == 1;

                    #[cfg(rev = "225")]
                    let child_count = buf.g1() as usize;
                    #[cfg(since_244)]
                    let child_count = buf.g2() as usize;
                    let mut child_x = Vec::with_capacity(child_count);
                    let mut child_y = Vec::with_capacity(child_count);
                    for _ in 0..child_count {
                        child_x.push(buf.g2s());
                        child_y.push(buf.g2s());
                    }
                    com.child_x = Some(child_x.into_boxed_slice());
                    com.child_y = Some(child_y.into_boxed_slice());
                }
                IfComponentType::Inv => {
                    com.draggable = buf.g1() == 1;
                    com.operable = buf.g1() == 1;
                    com.usable = buf.g1() == 1;
                    #[cfg(since_245_2)]
                    {
                        com.swappable = buf.g1() == 1;
                    }
                    com.margin_x = buf.g1();
                    com.margin_y = buf.g1();

                    let mut slot_x = vec![0; 20];
                    let mut slot_y = vec![0; 20];
                    let mut slot_graphic = vec![None; 20].into_boxed_slice();
                    for i in 0..20 {
                        if buf.g1() == 1 {
                            slot_x[i] = buf.g2s();
                            slot_y[i] = buf.g2s();
                            slot_graphic[i] = Some(buf.gjstr(10).into_boxed_str());
                        }
                    }
                    com.inventory_slot_offset_x = Some(slot_x.into_boxed_slice());
                    com.inventory_slot_offset_y = Some(slot_y.into_boxed_slice());
                    com.inventory_slot_graphic = Some(slot_graphic);

                    let mut options = Vec::with_capacity(5);
                    for _ in 0..5 {
                        let s = buf.gjstr(10).into_boxed_str();
                        options.push(if s.is_empty() { None } else { Some(s) });
                    }
                    com.iop = Some(options.into_boxed_slice());
                }
                IfComponentType::Rect => {
                    com.fill = buf.g1() == 1;
                }
                IfComponentType::Text => {
                    com.center = buf.g1() == 1;
                    com.font = buf.g1();
                    com.shadowed = buf.g1() == 1;
                    com.text = Some(buf.gjstr(10).into_boxed_str());
                    com.active_text = Some(buf.gjstr(10).into_boxed_str());
                }
                IfComponentType::Graphic => {
                    com.graphic = Some(buf.gjstr(10).into_boxed_str());
                    com.active_graphic = Some(buf.gjstr(10).into_boxed_str());
                }
                IfComponentType::Model => {
                    let m = buf.g1() as i32;
                    com.model = if m != 0 {
                        ((m - 1) << 8) + buf.g1() as i32
                    } else {
                        0
                    };
                    let am = buf.g1() as i32;
                    com.active_model = if am != 0 {
                        ((am - 1) << 8) + buf.g1() as i32
                    } else {
                        0
                    };
                    let a = buf.g1() as i32;
                    com.anim = if a == 0 {
                        -1
                    } else {
                        ((a - 1) << 8) + buf.g1() as i32
                    };
                    let aa = buf.g1() as i32;
                    com.active_anim = if aa == 0 {
                        -1
                    } else {
                        ((aa - 1) << 8) + buf.g1() as i32
                    };
                    com.zoom = buf.g2();
                    com.xan = buf.g2();
                    com.yan = buf.g2();
                }
                IfComponentType::InvText => {
                    com.center = buf.g1() == 1;
                    com.font = buf.g1();
                    com.shadowed = buf.g1() == 1;
                    com.colour = buf.g4s();
                    com.margin_x = buf.g2s() as u8;
                    com.margin_y = buf.g2s() as u8;
                    com.operable = buf.g1() == 1;

                    let mut options = Vec::with_capacity(5);
                    for _ in 0..5 {
                        let s = buf.gjstr(10).into_boxed_str();
                        options.push(if s.is_empty() { None } else { Some(s) });
                    }
                    com.iop = Some(options.into_boxed_slice());
                }
            }

            if com.com_type == IfComponentType::Rect || com.com_type == IfComponentType::Text {
                com.colour = buf.g4s();
                com.active_colour = buf.g4s();
                com.over_colour = buf.g4s();
                #[cfg(since_245_2)]
                {
                    com.active_over_colour = buf.g4s();
                }
            }

            if com.button_type == IfButtonType::Target || com.com_type == IfComponentType::Inv {
                com.action_verb = Some(buf.gjstr(10).into_boxed_str());
                com.action = Some(buf.gjstr(10).into_boxed_str());
                com.action_target = buf.g2();
            }

            if matches!(
                com.button_type,
                IfButtonType::Normal
                    | IfButtonType::Toggle
                    | IfButtonType::Select
                    | IfButtonType::Pause
            ) {
                com.option = Some(buf.gjstr(10).into_boxed_str());
            }

            if let Some(name) = &com.com_name
                && !name.is_empty()
            {
                names.insert(name.clone(), id);
            }

            if (id as usize) < types.len() {
                types[id as usize] = Some(Box::new(IfType::from(com)));
            }
        }

        IfTypeProvider { names, types }
    }

    pub fn get_by_id(&self, id: u16) -> Option<&IfType> {
        self.types.get(id as usize).and_then(|t| t.as_deref())
    }

    pub fn get_by_debugname(&self, name: &str) -> Option<&IfType> {
        self.names.get(name).and_then(|&id| self.get_by_id(id))
    }

    pub fn count(&self) -> usize {
        self.types.len()
    }
}
