//! The disassembler of the event scripts.
//!
//! The game dispatches 105 opcodes; the ones that are not dispatched are not guesses, they are unknowns that
//! stop the sweep. The listing keeps every byte it could read, and names the offsets that are the target of a
//! jump or the entry of a section.

use crate::error::{DwExtractError, Result};
use crate::formats::script::scn_file::{read_script_header, ScriptHeader};
use crate::formats::script::text;

pub const END_SCRIPT_OPCODE: u8 = 0xFF;
const SET_SELECTION: u8 = 0x10;
const SELECTION_END_MARKER: u8 = 0x12;
const SWITCH: u8 = 0x18;
const IF: u8 = 0x19;
const SHOW_TEXTBOX: u8 = 0x1A;
const DATA: u8 = 0x6B;
/// DATA blocks end with 0xFF.
const DATA_END: u8 = 0xFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentKind {
    Pad,
    U8,
    U16,
    S16,
    S32,
    Pstat,
    Trigger,
    Entity,
    Label,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    pub name: String,
    pub kind: ArgumentKind,
    pub value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionTest {
    Trigger,
    Pstat,
    GotoIfTrue,
    GotoIfFalse,
    Stat,
    Card,
    Move,
    Condition,
    ItemCount,
    Money,
}

impl Default for ConditionTest {
    fn default() -> Self {
        ConditionTest::Trigger
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combine {
    Replace,
    And,
    Or,
}

impl Default for Combine {
    fn default() -> Self {
        Combine::Replace
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Condition {
    pub code: u8,
    pub test: ConditionTest,
    pub combine: Combine,
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PadByte {
    /// The offset of the pad, relative to the start of the instruction.
    pub offset: u8,
    pub value: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Instruction {
    pub offset: u32,
    /// The bytes of the instruction, its pads included.
    pub size: u32,
    pub opcode: u8,
    pub arguments: Vec<Argument>,
    pub targets: Vec<u16>,
    pub conditions: Vec<Condition>,
    pub text: Option<String>,
    pub data: Vec<u8>,
    pub selection_end_marker: bool,
    pub pads: Vec<PadByte>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Disassembly {
    pub header: ScriptHeader,
    pub instructions: Vec<Instruction>,
    pub code_end: u32,
    /// The offsets that are the entry of a section or the target of a jump.
    pub labels: Vec<u16>,
    /// The instructions that no flow from a section entry reaches.
    pub unreachable: usize,
    /// What the interpreter would not run, or would not read.
    pub unknowns: Vec<String>,
}

/// Where execution can go after an instruction, for the reachability pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    /// The next instruction.
    Next,
    /// The target only.
    Goto,
    /// The target, and the next instruction when it returns.
    Call,
    /// Nowhere: return, end, a jump to another script, DATA.
    Stop,
    /// The cases only.
    Switch,
    /// The targets and the next instruction.
    Branch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Field {
    kind: ArgumentKind,
    name: &'static str,
}

const fn pad() -> Field {
    Field { kind: ArgumentKind::Pad, name: "" }
}

const fn u8_field(name: &'static str) -> Field {
    Field { kind: ArgumentKind::U8, name }
}

const fn u16_field(name: &'static str) -> Field {
    Field { kind: ArgumentKind::U16, name }
}

const fn s16_field(name: &'static str) -> Field {
    Field { kind: ArgumentKind::S16, name }
}

const fn s32_field(name: &'static str) -> Field {
    Field { kind: ArgumentKind::S32, name }
}

const fn pstat_field(name: &'static str) -> Field {
    Field { kind: ArgumentKind::Pstat, name }
}

const fn trigger_field() -> Field {
    Field { kind: ArgumentKind::Trigger, name: "trigger" }
}

const fn entity_field(name: &'static str) -> Field {
    Field { kind: ArgumentKind::Entity, name }
}

const fn label_field() -> Field {
    Field { kind: ArgumentKind::Label, name: "target" }
}

struct OpcodeInfo {
    code: u8,
    name: &'static str,
    flow: Flow,
    fields: &'static [Field],
}

/// The opcodes tickScript dispatches (0x10-0x3F, 0x46-0x58, 0x5A-0x5F, 0x64-0x7E, 0xFB-0xFF), named after
/// include/dw/script.h. The layouts are the fixed operands; SET_SELECTION, SWITCH, IF, SHOW_TEXTBOX and DATA go on
/// with variable parts that the decoder reads.
static OPCODES: &[OpcodeInfo] = &[
    OpcodeInfo { code: 0x10, name: "set_selection", flow: Flow::Branch, fields: &[u8_field("count")] },
    OpcodeInfo { code: 0x11, name: "unused_11", flow: Flow::Next, fields: &[] },
    OpcodeInfo { code: 0x12, name: "unused_12", flow: Flow::Next, fields: &[pad()] },
    OpcodeInfo { code: 0x13, name: "jump_and_link", flow: Flow::Call, fields: &[pad(), label_field()] },
    OpcodeInfo { code: 0x14, name: "jump_to_file_and_link", flow: Flow::Next, fields: &[pad(), u16_field("script"), u16_field("section")] },
    OpcodeInfo { code: 0x15, name: "jump_return", flow: Flow::Stop, fields: &[pad()] },
    OpcodeInfo { code: 0x16, name: "jump_to", flow: Flow::Goto, fields: &[pad(), label_field()] },
    OpcodeInfo { code: 0x17, name: "jump_to_file", flow: Flow::Stop, fields: &[pad(), u16_field("script"), u16_field("section")] },
    OpcodeInfo { code: 0x18, name: "switch", flow: Flow::Switch, fields: &[pstat_field("pstat"), u16_field("count")] },
    OpcodeInfo { code: 0x19, name: "if", flow: Flow::Branch, fields: &[pad()] },
    OpcodeInfo { code: 0x1A, name: "show_textbox", flow: Flow::Next, fields: &[pad()] },
    OpcodeInfo { code: 0x1B, name: "set_dialog_owner", flow: Flow::Next, fields: &[entity_field("entity")] },
    OpcodeInfo { code: 0x1C, name: "set_trigger", flow: Flow::Next, fields: &[pad(), trigger_field()] },
    OpcodeInfo { code: 0x1D, name: "unset_trigger", flow: Flow::Next, fields: &[pad(), trigger_field()] },
    OpcodeInfo { code: 0x1E, name: "set_pstat", flow: Flow::Next, fields: &[pad(), pstat_field("pstat"), u8_field("value")] },
    OpcodeInfo { code: 0x1F, name: "add_to_pstat", flow: Flow::Next, fields: &[pad(), pstat_field("pstat"), u8_field("value")] },
    OpcodeInfo { code: 0x20, name: "reduce_pstat", flow: Flow::Next, fields: &[pad(), pstat_field("pstat"), u8_field("value")] },
    OpcodeInfo { code: 0x21, name: "store_map_id", flow: Flow::Next, fields: &[pstat_field("pstat")] },
    OpcodeInfo { code: 0x22, name: "store_digimon_type", flow: Flow::Next, fields: &[pstat_field("pstat")] },
    OpcodeInfo { code: 0x23, name: "set_inventory_size", flow: Flow::Next, fields: &[u8_field("size")] },
    OpcodeInfo { code: 0x24, name: "store_random", flow: Flow::Next, fields: &[pad(), pstat_field("pstat"), u8_field("max")] },
    OpcodeInfo { code: 0x25, name: "store_date", flow: Flow::Next, fields: &[pstat_field("pstat")] },
    OpcodeInfo { code: 0x26, name: "set_textbox_size", flow: Flow::Next, fields: &[entity_field("entity"), u8_field("columns"), u8_field("rows")] },
    OpcodeInfo { code: 0x27, name: "fadeout_hud", flow: Flow::Next, fields: &[u8_field("box")] },
    OpcodeInfo { code: 0x28, name: "give_item", flow: Flow::Next, fields: &[pad(), u8_field("item"), u8_field("amount")] },
    OpcodeInfo { code: 0x29, name: "remove_item", flow: Flow::Next, fields: &[pad(), u8_field("item"), u8_field("amount")] },
    OpcodeInfo { code: 0x2A, name: "add_money", flow: Flow::Next, fields: &[pad(), s32_field("amount")] },
    OpcodeInfo { code: 0x2B, name: "reduce_money", flow: Flow::Next, fields: &[pad(), s32_field("amount")] },
    OpcodeInfo { code: 0x2C, name: "compare_date", flow: Flow::Next, fields: &[pstat_field("pstat"), trigger_field(), u8_field("compare"), u8_field("year"), u8_field("day"), u8_field("hour"), u8_field("minute"), pad()] },
    OpcodeInfo { code: 0x2D, name: "learn_move", flow: Flow::Next, fields: &[u8_field("move")] },
    OpcodeInfo { code: 0x2E, name: "unused_2e", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x2F, name: "give_card", flow: Flow::Next, fields: &[u8_field("card")] },
    OpcodeInfo { code: 0x30, name: "take_card", flow: Flow::Next, fields: &[u8_field("card")] },
    OpcodeInfo { code: 0x31, name: "set_merit", flow: Flow::Next, fields: &[pad(), u16_field("merit")] },
    OpcodeInfo { code: 0x32, name: "add_merit", flow: Flow::Next, fields: &[pad(), u16_field("merit")] },
    OpcodeInfo { code: 0x33, name: "reduce_merit", flow: Flow::Next, fields: &[pad(), u16_field("merit")] },
    OpcodeInfo { code: 0x34, name: "set_stat", flow: Flow::Next, fields: &[u8_field("stat"), u16_field("value")] },
    OpcodeInfo { code: 0x35, name: "add_stat", flow: Flow::Next, fields: &[u8_field("stat"), u16_field("value")] },
    OpcodeInfo { code: 0x36, name: "reduce_stat", flow: Flow::Next, fields: &[u8_field("stat"), u16_field("value")] },
    OpcodeInfo { code: 0x37, name: "advance_to_date_at", flow: Flow::Next, fields: &[pstat_field("pstat")] },
    OpcodeInfo { code: 0x38, name: "add_minutes_to_date_at", flow: Flow::Next, fields: &[pstat_field("pstat"), s32_field("minutes")] },
    OpcodeInfo { code: 0x39, name: "add_minutes_to_date_at_2", flow: Flow::Next, fields: &[pstat_field("pstat"), s32_field("minutes")] },
    OpcodeInfo { code: 0x3A, name: "unused_3a", flow: Flow::Next, fields: &[pad(), u8_field("unknown02"), u8_field("unknown03")] },
    OpcodeInfo { code: 0x3B, name: "unused_3b", flow: Flow::Next, fields: &[pad(), u8_field("unknown02"), u8_field("unknown03")] },
    OpcodeInfo { code: 0x3C, name: "unused_3c", flow: Flow::Next, fields: &[pad(), u8_field("unknown02"), u8_field("unknown03")] },
    OpcodeInfo { code: 0x3D, name: "unused_3d", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x3E, name: "unused_3e", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x3F, name: "store_digimon_value", flow: Flow::Next, fields: &[pad(), pstat_field("source"), pstat_field("destination")] },
    OpcodeInfo { code: 0x46, name: "load_digimon", flow: Flow::Next, fields: &[u8_field("digimon")] },
    OpcodeInfo { code: 0x47, name: "set_digimon", flow: Flow::Next, fields: &[u8_field("digimon"), u8_field("slot"), u8_field("autotalk")] },
    OpcodeInfo { code: 0x48, name: "unload_entity", flow: Flow::Next, fields: &[entity_field("entity")] },
    OpcodeInfo { code: 0x49, name: "call_digimon_routine", flow: Flow::Next, fields: &[u8_field("routine")] },
    OpcodeInfo { code: 0x4A, name: "wait_for_entity", flow: Flow::Next, fields: &[entity_field("entity")] },
    OpcodeInfo { code: 0x4B, name: "warp_to", flow: Flow::Next, fields: &[u8_field("screen"), u8_field("exit"), u8_field("section")] },
    OpcodeInfo { code: 0x4C, name: "entity_look_at_entity", flow: Flow::Next, fields: &[pad(), entity_field("entity"), entity_field("target_entity")] },
    OpcodeInfo { code: 0x4D, name: "entity_set_rotation", flow: Flow::Next, fields: &[entity_field("entity"), s16_field("rotation")] },
    OpcodeInfo { code: 0x4E, name: "entity_walk_to", flow: Flow::Next, fields: &[entity_field("entity"), s16_field("x"), s16_field("z"), u8_field("animation"), u8_field("unknown07")] },
    OpcodeInfo { code: 0x4F, name: "move_camera_to", flow: Flow::Next, fields: &[u8_field("speed"), s16_field("x"), s16_field("z")] },
    OpcodeInfo { code: 0x50, name: "move_camera_to_entity", flow: Flow::Next, fields: &[pad(), entity_field("entity"), u8_field("speed")] },
    OpcodeInfo { code: 0x51, name: "entity_walk_to_entity", flow: Flow::Next, fields: &[entity_field("entity"), u8_field("animation"), entity_field("target_entity")] },
    OpcodeInfo { code: 0x52, name: "entity_walk_to_with_camera", flow: Flow::Next, fields: &[entity_field("entity"), s16_field("x"), s16_field("z"), u8_field("animation"), u8_field("unknown07")] },
    OpcodeInfo { code: 0x53, name: "entity_walk_to_entity_with_camera", flow: Flow::Next, fields: &[entity_field("entity"), u8_field("animation"), entity_field("target_entity")] },
    OpcodeInfo { code: 0x54, name: "reset_entity_origin", flow: Flow::Next, fields: &[entity_field("entity")] },
    OpcodeInfo { code: 0x55, name: "set_textbox_origin", flow: Flow::Next, fields: &[pad(), s16_field("x"), s16_field("y"), s16_field("z")] },
    OpcodeInfo { code: 0x56, name: "play_animation", flow: Flow::Next, fields: &[pad(), entity_field("entity"), u8_field("animation")] },
    OpcodeInfo { code: 0x57, name: "set_obj_visibility", flow: Flow::Next, fields: &[pad(), u8_field("object"), u8_field("visible")] },
    OpcodeInfo { code: 0x58, name: "teleport", flow: Flow::Next, fields: &[pstat_field("pstat")] },
    OpcodeInfo { code: 0x5A, name: "play_sound", flow: Flow::Next, fields: &[pad(), u8_field("bank"), u8_field("sound")] },
    OpcodeInfo { code: 0x5B, name: "unused_5b", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x5C, name: "unused_5c", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x5D, name: "set_bgm", flow: Flow::Next, fields: &[u8_field("track")] },
    OpcodeInfo { code: 0x5E, name: "stop_bgm", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x5F, name: "unused_5f", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x64, name: "call_routine", flow: Flow::Next, fields: &[u8_field("routine")] },
    OpcodeInfo { code: 0x65, name: "remove_condition", flow: Flow::Next, fields: &[u8_field("mask")] },
    OpcodeInfo { code: 0x66, name: "start_battle", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x67, name: "delay", flow: Flow::Next, fields: &[pad(), u16_field("frames")] },
    OpcodeInfo { code: 0x68, name: "set_textbox_mode", flow: Flow::Next, fields: &[pad(), u8_field("mode"), u8_field("frames")] },
    OpcodeInfo { code: 0x69, name: "deal_damage", flow: Flow::Next, fields: &[pstat_field("pstat")] },
    OpcodeInfo { code: 0x6A, name: "set_autotalk", flow: Flow::Next, fields: &[pad(), entity_field("entity"), u8_field("enabled")] },
    OpcodeInfo { code: 0x6B, name: "data", flow: Flow::Stop, fields: &[pad()] },
    OpcodeInfo { code: 0x6C, name: "entity_move_to", flow: Flow::Next, fields: &[entity_field("entity"), s16_field("x"), s16_field("z"), u8_field("speed"), u8_field("unknown07")] },
    OpcodeInfo { code: 0x6D, name: "entity_move_to_entity", flow: Flow::Next, fields: &[entity_field("entity"), entity_field("target_entity"), u8_field("speed")] },
    OpcodeInfo { code: 0x6E, name: "entity_move_to_with_camera", flow: Flow::Next, fields: &[entity_field("entity"), s16_field("x"), s16_field("z"), u8_field("speed"), u8_field("unknown07")] },
    OpcodeInfo { code: 0x6F, name: "entity_move_to_entity_with_camera", flow: Flow::Next, fields: &[entity_field("entity"), entity_field("target_entity"), u8_field("speed")] },
    OpcodeInfo { code: 0x70, name: "rotate_3d_object", flow: Flow::Next, fields: &[entity_field("entity"), u8_field("unknown02"), u8_field("target")] },
    OpcodeInfo { code: 0x71, name: "move_object_to", flow: Flow::Next, fields: &[u8_field("slot"), entity_field("entity"), u8_field("speed"), entity_field("target_entity"), pad(), s16_field("x"), s16_field("z")] },
    OpcodeInfo { code: 0x72, name: "entity_move_to_axis", flow: Flow::Next, fields: &[entity_field("entity"), s16_field("position"), u8_field("axis"), u8_field("speed")] },
    OpcodeInfo { code: 0x73, name: "entity_move_to_axis_with_camera", flow: Flow::Next, fields: &[entity_field("entity"), s16_field("position"), u8_field("axis"), u8_field("speed")] },
    OpcodeInfo { code: 0x74, name: "spawn_item", flow: Flow::Next, fields: &[u8_field("item"), s16_field("x"), s16_field("z")] },
    OpcodeInfo { code: 0x75, name: "spawn_chest", flow: Flow::Next, fields: &[u8_field("item"), s16_field("x"), s16_field("y"), s16_field("z"), s16_field("w"), trigger_field()] },
    OpcodeInfo { code: 0x76, name: "spawn_boulder", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x77, name: "move_boulder", flow: Flow::Next, fields: &[u8_field("unknown01"), s16_field("x"), s16_field("z")] },
    OpcodeInfo { code: 0x78, name: "despawn_boulder", flow: Flow::Next, fields: &[u8_field("unknown01")] },
    OpcodeInfo { code: 0x79, name: "unload_digimon", flow: Flow::Next, fields: &[u8_field("model")] },
    OpcodeInfo { code: 0x7A, name: "copy_pstat", flow: Flow::Next, fields: &[pad(), pstat_field("source"), pstat_field("destination")] },
    OpcodeInfo { code: 0x7B, name: "section_on_exit", flow: Flow::Next, fields: &[u8_field("section")] },
    OpcodeInfo { code: 0x7C, name: "set_rect_impassable", flow: Flow::Next, fields: &[u8_field("unknown01"), s16_field("x"), s16_field("z"), u8_field("width"), u8_field("height")] },
    OpcodeInfo { code: 0x7D, name: "spawn_sprite_at_location", flow: Flow::Next, fields: &[u8_field("sprite"), s16_field("x"), s16_field("y"), s16_field("z"), s16_field("w")] },
    OpcodeInfo { code: 0x7E, name: "spawn_sprite_at_entity", flow: Flow::Next, fields: &[entity_field("entity"), u8_field("node"), u8_field("sprite")] },
    OpcodeInfo { code: 0xFB, name: "set_script", flow: Flow::Next, fields: &[pad(), u16_field("script"), u16_field("map")] },
    OpcodeInfo { code: 0xFC, name: "unused_fc", flow: Flow::Next, fields: &[] },
    OpcodeInfo { code: 0xFD, name: "unused_fd", flow: Flow::Next, fields: &[] },
    OpcodeInfo { code: 0xFE, name: "end_section", flow: Flow::Stop, fields: &[] },
    OpcodeInfo { code: 0xFF, name: "end_script", flow: Flow::Stop, fields: &[] },
];

const COMPARE_NAMES: [&str; 6] = ["eq", "ne", "ge", "le", "gt", "lt"];

/// The names of the comparisons, in the order the interpreter uses.
pub fn compare_names() -> &'static [&'static str; 6] {
    &COMPARE_NAMES
}

fn find_opcode(code: u8) -> Option<&'static OpcodeInfo> {
    OPCODES.iter().find(|info| info.code == code)
}

/// The name of an opcode; empty when the interpreter does not dispatch it.
pub fn opcode_name(opcode: u8) -> &'static str {
    find_opcode(opcode).map(|info| info.name).unwrap_or("")
}

fn field_size(kind: ArgumentKind) -> usize {
    match kind {
        ArgumentKind::U16 | ArgumentKind::S16 | ArgumentKind::Trigger | ArgumentKind::Label => 2,
        ArgumentKind::S32 => 4,
        ArgumentKind::Pad | ArgumentKind::U8 | ArgumentKind::Pstat | ArgumentKind::Entity => 1,
    }
}

/// Reads the fields of an instruction one after another.
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], at: usize) -> Self {
        Self { data, at }
    }

    fn at(&self) -> usize {
        self.at
    }

    /// Reads a field; nothing when it runs past the end.
    fn field(&mut self, kind: ArgumentKind) -> Option<i32> {
        let size = field_size(kind);
        if self.at + size > self.data.len() {
            return None;
        }
        let value = match size {
            1 => i32::from(self.data[self.at]),
            2 => {
                let bytes = [self.data[self.at], self.data[self.at + 1]];
                if kind == ArgumentKind::S16 {
                    i32::from(i16::from_le_bytes(bytes))
                } else {
                    i32::from(u16::from_le_bytes(bytes))
                }
            }
            _ => i32::from_le_bytes([
                self.data[self.at],
                self.data[self.at + 1],
                self.data[self.at + 2],
                self.data[self.at + 3],
            ]),
        };
        self.at += size;
        Some(value)
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.at).copied()
    }

    fn skip(&mut self, count: usize) {
        self.at += count;
    }
}

/// Reads a field into the instruction: a pad that is not 0 is kept, other fields become arguments.
fn read_field(reader: &mut Reader, instruction: &mut Instruction, kind: ArgumentKind, name: &str) -> bool {
    let offset = reader.at() as u32;
    let value = match reader.field(kind) {
        Some(value) => value,
        None => return false,
    };

    if kind == ArgumentKind::Pad {
        if value != 0 {
            instruction.pads.push(PadByte {
                offset: (offset - instruction.offset) as u8,
                value: value as u8,
            });
        }
    } else if !name.is_empty() {
        instruction.arguments.push(Argument {
            name: name.to_string(),
            kind,
            value,
        });
    }
    true
}

/// Reads the entries of an IF after its pad, up to the 0x19 entry that ends it.
fn decode_if(reader: &mut Reader, instruction: &mut Instruction) -> Result<()> {
    loop {
        let code = match reader.peek() {
            Some(code) => code,
            None => {
                return Err(DwExtractError::InvalidFormat(
                    "the IF runs past the end of the script".into(),
                ))
            }
        };

        if code == IF {
            reader.skip(1);
            if !read_field(reader, instruction, ArgumentKind::Pad, "") {
                return Err(DwExtractError::InvalidFormat(
                    "the IF runs past the end of the script".into(),
                ));
            }
            return Ok(());
        }

        let mut condition = Condition {
            code,
            test: ConditionTest::Trigger,
            combine: Combine::Replace,
            arguments: Vec::new(),
        };
        if code & 0x80 != 0 {
            condition.combine = Combine::And;
        } else if code & 0x40 != 0 {
            condition.combine = Combine::Or;
        }
        let low = u32::from(code & 0x07);

        let payload: &[Field] = match code & 0x38 {
            0x00 => {
                condition.test = ConditionTest::Trigger;
                &[trigger_field()]
            }
            0x08 => {
                condition.test = ConditionTest::Pstat;
                &[pstat_field("pstat"), u8_field("value")]
            }
            0x10 => {
                condition.test = ConditionTest::GotoIfTrue;
                &[label_field()]
            }
            0x18 => {
                condition.test = ConditionTest::GotoIfFalse;
                &[label_field()]
            }
            0x20 => match low {
                0 => {
                    condition.test = ConditionTest::Stat;
                    &[u8_field("stat"), u8_field("compare"), u16_field("value")]
                }
                1 => {
                    condition.test = ConditionTest::Card;
                    &[u8_field("card"), u8_field("compare"), u16_field("value")]
                }
                2 => {
                    condition.test = ConditionTest::Move;
                    &[u8_field("move"), u8_field("negate")]
                }
                3 => {
                    condition.test = ConditionTest::Condition;
                    &[u8_field("mask"), u8_field("negate")]
                }
                4 => {
                    condition.test = ConditionTest::ItemCount;
                    &[u8_field("item"), u8_field("compare"), u16_field("value")]
                }
                5 => {
                    condition.test = ConditionTest::Money;
                    &[pad(), u8_field("compare"), s32_field("value")]
                }
                _ => {
                    return Err(DwExtractError::InvalidFormat(format!(
                        "IF test {:#04x} is not one the interpreter has",
                        code
                    )))
                }
            },
            _ => {
                return Err(DwExtractError::InvalidFormat(format!(
                    "IF test {:#04x} is not one the interpreter has",
                    code
                )))
            }
        };

        if condition.test == ConditionTest::Pstat && low >= COMPARE_NAMES.len() as u32 {
            return Err(DwExtractError::InvalidFormat(format!(
                "IF test {:#04x} compares with operator {}, which has no case",
                code, low
            )));
        }

        // The arguments of an entry go to the condition, not to the instruction.
        reader.skip(1);
        if !read_field(reader, instruction, ArgumentKind::Pad, "") {
            return Err(DwExtractError::InvalidFormat(
                "the IF runs past the end of the script".into(),
            ));
        }
        for field in payload {
            let offset = reader.at() as u32;
            let value = match reader.field(field.kind) {
                Some(value) => value,
                None => {
                    return Err(DwExtractError::InvalidFormat(
                        "the IF runs past the end of the script".into(),
                    ))
                }
            };
            if field.kind == ArgumentKind::Pad {
                if value != 0 {
                    instruction.pads.push(PadByte {
                        offset: (offset - instruction.offset) as u8,
                        value: value as u8,
                    });
                }
            } else {
                condition.arguments.push(Argument {
                    name: field.name.to_string(),
                    kind: field.kind,
                    value,
                });
            }
        }

        let bad_compare = condition
            .arguments
            .iter()
            .find(|argument| argument.name == "compare" && (argument.value & 0x07) >= COMPARE_NAMES.len() as i32);
        if let Some(argument) = bad_compare {
            return Err(DwExtractError::InvalidFormat(format!(
                "IF test {:#04x} compares with operator {}, which has no case",
                code,
                argument.value & 0x07
            )));
        }

        instruction.conditions.push(condition);
    }
}

/// Decodes the text at the reader, and keeps it in the instruction.
fn decode_text(script: &[u8], reader: &mut Reader, instruction: &mut Instruction) -> Result<()> {
    let text = text::decode_script_text(&script[reader.at()..]).map_err(|error| {
        DwExtractError::InvalidFormat(format!("text: {}", error))
    })?;
    instruction.text = Some(text.text);
    reader.skip(text.size);
    Ok(())
}

/// Decodes the instruction at an offset into `instruction`, its size included.
pub fn decode_instruction(script: &[u8], instruction: &mut Instruction) -> Result<()> {
    let opcode = instruction.opcode;
    let info = match find_opcode(opcode) {
        Some(info) => info,
        None => {
            return Err(DwExtractError::InvalidFormat(format!(
                "opcode {:#04x} is not one the interpreter dispatches",
                opcode
            )))
        }
    };

    let mut reader = Reader::new(script, instruction.offset as usize + 1);
    for field in info.fields {
        if !read_field(&mut reader, instruction, field.kind, field.name) {
            return Err(DwExtractError::InvalidFormat(format!(
                "the operands of {} run past the end of the script",
                info.name
            )));
        }
    }

    match opcode {
        SET_SELECTION => {
            // The options, then a SHOW_TEXTBOX word that scriptShowSelection counts as one more table entry.
            let count = match instruction.arguments.pop() {
                Some(argument) => argument.value.max(0) as usize,
                None => 0,
            };
            for _ in 0..count {
                match reader.field(ArgumentKind::U16) {
                    Some(target) => instruction.targets.push(target as u16),
                    None => {
                        return Err(DwExtractError::InvalidFormat(format!(
                            "the operands of {} run past the end of the script",
                            info.name
                        )))
                    }
                }
            }
            if reader.peek() != Some(SHOW_TEXTBOX) {
                return Err(DwExtractError::InvalidFormat(
                    "SET_SELECTION is not followed by its SHOW_TEXTBOX word".into(),
                ));
            }
            reader.skip(1);
            if !read_field(&mut reader, instruction, ArgumentKind::Pad, "") {
                return Err(DwExtractError::InvalidFormat(format!(
                    "the operands of {} run past the end of the script",
                    info.name
                )));
            }
            decode_text(script, &mut reader, instruction)?;
            if reader.peek() == Some(SELECTION_END_MARKER) && script.get(reader.at() + 1) == Some(&0) {
                instruction.selection_end_marker = true;
                reader.skip(2);
            }
        }
        SWITCH => {
            let count = match instruction.arguments.pop() {
                Some(argument) => argument.value.max(0) as usize,
                None => 0,
            };
            for _ in 0..count {
                match reader.field(ArgumentKind::U16) {
                    Some(target) => instruction.targets.push(target as u16),
                    None => {
                        return Err(DwExtractError::InvalidFormat(format!(
                            "the operands of {} run past the end of the script",
                            info.name
                        )))
                    }
                }
            }
        }
        IF => decode_if(&mut reader, instruction)?,
        SHOW_TEXTBOX => decode_text(script, &mut reader, instruction)?,
        DATA => loop {
            match reader.peek() {
                Some(byte) => {
                    instruction.data.push(byte);
                    reader.skip(1);
                    if byte == DATA_END {
                        break;
                    }
                }
                None => {
                    return Err(DwExtractError::InvalidFormat(
                        "DATA runs past the end of the script".into(),
                    ))
                }
            }
        },
        _ => {}
    }

    instruction.size = reader.at() as u32 - instruction.offset;
    Ok(())
}

/// Every offset an instruction can jump to.
pub fn targets_of(instruction: &Instruction) -> Vec<u16> {
    let mut targets = instruction.targets.clone();
    let mut add_labels = |arguments: &[Argument]| {
        for argument in arguments {
            if argument.kind == ArgumentKind::Label {
                targets.push(argument.value as u16);
            }
        }
    };
    add_labels(&instruction.arguments);
    for condition in &instruction.conditions {
        add_labels(&condition.arguments);
    }
    targets
}

/// Counts the instructions that no flow from a section entry reaches.
fn count_unreachable(disassembly: &Disassembly) -> usize {
    let instructions = &disassembly.instructions;
    let index_of = |offset: u32| instructions.iter().position(|instruction| instruction.offset == offset);

    let mut reached = vec![false; instructions.len()];
    let mut work: Vec<u32> = disassembly
        .header
        .sections
        .iter()
        .map(|section| u32::from(section.offset))
        .collect();

    while let Some(offset) = work.pop() {
        let mut index = index_of(offset);
        while let Some(current) = index {
            if reached[current] {
                break;
            }
            reached[current] = true;
            let instruction = &instructions[current];
            work.extend(targets_of(instruction).into_iter().map(u32::from));
            let flow = find_opcode(instruction.opcode).map(|info| info.flow).unwrap_or(Flow::Next);
            if matches!(flow, Flow::Goto | Flow::Stop | Flow::Switch) {
                break;
            }
            index = if current + 1 < instructions.len() {
                Some(current + 1)
            } else {
                None
            };
        }
    }

    reached.iter().filter(|was_reached| !**was_reached).count()
}

/// Disassembles a script: the bytes of one script, not of a file of scripts.
pub fn disassemble_script(script: &[u8]) -> Result<Disassembly> {
    let header = read_script_header(script)?;

    let mut disassembly = Disassembly {
        header,
        instructions: Vec::new(),
        code_end: 0,
        labels: Vec::new(),
        unreachable: 0,
        unknowns: Vec::new(),
    };

    // The section entries and the targets of the jumps are the labels.
    let mut labels: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    let mut farthest = 0u32;
    for section in &disassembly.header.sections {
        labels.insert(section.offset);
        farthest = farthest.max(u32::from(section.offset));
    }

    let mut at = u32::from(disassembly.header.code_start);
    loop {
        let opcode = match script.get(at as usize) {
            Some(opcode) => *opcode,
            None => {
                disassembly.unknowns.push(format!(
                    "{:#06x}: the code reaches the end of the script without an END_SCRIPT past its targets",
                    at
                ));
                break;
            }
        };

        let mut instruction = Instruction {
            offset: at,
            opcode,
            ..Instruction::default()
        };

        if let Err(error) = decode_instruction(script, &mut instruction) {
            disassembly.unknowns.push(format!(
                "{:#06x}: {}; {} bytes from here are not decoded",
                at,
                error,
                script.len() - at as usize
            ));
            break;
        }

        at += instruction.size;
        let mut desync = false;
        if at % 2 != 0 {
            // The pad after a 1-byte instruction. DATA is read a byte at a time, so the arena scripts pack their
            // blocks one after another at any offset; anything else at an odd offset would break the aligned reads.
            match script.get(at as usize) {
                Some(0) => {
                    instruction.size += 1;
                    at += 1;
                }
                Some(&DATA) => {}
                Some(_) => desync = true,
                None => desync = false,
            }
        }

        for target in targets_of(&instruction) {
            labels.insert(target);
            farthest = farthest.max(u32::from(target));
        }

        let end = instruction.opcode == END_SCRIPT_OPCODE && at > farthest;
        disassembly.instructions.push(instruction);

        if desync {
            disassembly.unknowns.push(format!(
                "{:#06x}: an instruction other than DATA starts at an odd offset; {} bytes from here are not decoded",
                at,
                script.len() - at as usize
            ));
            break;
        }

        if end {
            break;
        }
    }

    disassembly.code_end = at;
    disassembly.labels = labels.into_iter().collect();

    // Labels past code_end are left to the unknown that stopped the sweep.
    for offset in &disassembly.labels {
        if u32::from(*offset) >= disassembly.code_end {
            continue;
        }
        let on_instruction = disassembly
            .instructions
            .binary_search_by(|instruction| instruction.offset.cmp(&u32::from(*offset)))
            .is_ok();
        if !on_instruction {
            disassembly.unknowns.push(format!(
                "{:#06x}: a section or target that is not an instruction",
                offset
            ));
        }
    }

    disassembly.unreachable = count_unreachable(&disassembly);
    Ok(disassembly)
}

/// The listing of a script: every instruction, its operands and its targets.
pub fn disassembly_to_text(disassembly: &Disassembly) -> String {
    let mut out = String::new();
    out.push_str("SCN Disassembly\n");
    out.push_str("================\n\n");
    out.push_str(&format!("Scripts: {}\n", disassembly.header.sections.len()));
    out.push_str(&format!("Instructions: {}\n", disassembly.instructions.len()));
    out.push_str(&format!("Code end: 0x{:X}\n", disassembly.code_end));
    out.push_str(&format!("Labels: {}\n", disassembly.labels.len()));
    out.push_str(&format!("Unreachable: {}\n", disassembly.unreachable));
    for unknown in &disassembly.unknowns {
        out.push_str(&format!("Unknown: {}\n", unknown));
    }
    out.push('\n');

    for instruction in &disassembly.instructions {
        out.push_str(&format!(
            "{:#06x}: {}",
            instruction.offset,
            opcode_name(instruction.opcode)
        ));
        for argument in &instruction.arguments {
            out.push_str(&format!(" {}={}", argument.name, argument.value));
        }
        for target in &instruction.targets {
            out.push_str(&format!(" target={:#06x}", target));
        }
        for condition in &instruction.conditions {
            out.push_str(&format!(
                " if({:?},{:?})",
                condition.test, condition.combine
            ));
        }
        if let Some(text) = &instruction.text {
            out.push_str(&format!("\n  text: {}", text));
        }
        if instruction.selection_end_marker {
            out.push_str(" [selection_end]");
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::script::test_support::ScriptBuilder;

    /// A script with code at 12 that goes through every kind of operand, an IF, a SWITCH, a selection, a call, a
    /// SET_SCRIPT, both ends and packed DATA blocks after the first END_SCRIPT.
    fn every_kind_of_instruction() -> ScriptBuilder {
        let mut script = ScriptBuilder::new(&[(0xFE, 12), (0x05, 92)]);
        script.u8(&[0x1E, 0x11, 5, 7]); // 12 set_pstat, with a pad byte that is not 0
        script.u8(&[0x1C, 0x00]).u16(0x1234); // 16 set_trigger
        script.u8(&[0x2A, 0x00]).u32(0xFFFFFFFE); // 20 add_money -2
        script.u8(&[0x4D, 0xFC]).u16(0xFF38); // 26 entity_set_rotation partner -200
        script.u8(&[0x34, 0x03]).u16(0x10); // 30 set_stat
        script.u8(&[0x19, 0x00, 0x0A, 0x00, 5, 2, 0x18, 0x00]); // 34 if pstat 5 >= 2, else goto 92
        script.u16(92).u8(&[0x19, 0x00]);
        script.u8(&[0x1A, 0x00, 0x82, 0x60, 0x0D, 0x00, 0x00, 0x00]); // 46 show_textbox
        script.u8(&[0x13, 0x00]).u16(94); // 54 jump_and_link
        script.u8(&[0x18, 5]).u16(2).u16(66).u16(103); // 58 switch
        script.u8(&[0x10, 2]).u16(92).u16(98); // 66 set_selection
        script.u8(&[0x1A, 0x00, 0x82, 0x60, 0x0D, 0x00, 0x00, 0x00, 0x12, 0x00]);
        script.u8(&[0x16, 0x00]).u16(108); // 82 jump_to
        script.u8(&[0xFB, 0x00]).u16(3).u16(4); // 86 set_script
        script.u8(&[0xFE, 0x00, 0x15, 0x00, 0xFF, 0x00]); // 92 end_section, 94 jump_return, 96 end_script
        script.u8(&[0x6B, 0x00, 0x01, 0x02, 0xFF]); // 98 data
        script.u8(&[0x6B, 0x00, 0x03, 0xFF, 0x00]); // 103 data, then a pad
        script.u8(&[0xFF, 0x00]); // 108 end_script
        script
    }

    fn instruction_at(disassembly: &Disassembly, offset: u32) -> &Instruction {
        disassembly
            .instructions
            .iter()
            .find(|instruction| instruction.offset == offset)
            .unwrap_or_else(|| panic!("there is no instruction at {:#x}", offset))
    }

    /// The unknowns of a script with the given code after a single section at 8.
    fn unknowns_of(code: &[u8]) -> Vec<String> {
        let mut script = ScriptBuilder::new(&[(0xFE, 8)]);
        script.u8(code);
        disassemble_script(script.bytes()).unwrap().unknowns
    }

    #[test]
    fn the_opcodes_the_interpreter_dispatches_have_a_name() {
        assert_eq!(opcode_name(0x1E), "set_pstat");
        assert_eq!(opcode_name(0xFF), "end_script");
        assert_eq!(opcode_name(0x40), "");
        assert_eq!(OPCODES.len(), 105);
    }

    #[test]
    fn the_operand_kinds_have_their_size() {
        assert_eq!(field_size(ArgumentKind::U8), 1);
        assert_eq!(field_size(ArgumentKind::U16), 2);
        assert_eq!(field_size(ArgumentKind::S16), 2);
        assert_eq!(field_size(ArgumentKind::S32), 4);
        assert_eq!(field_size(ArgumentKind::Pad), 1);
        assert_eq!(field_size(ArgumentKind::Pstat), 1);
        assert_eq!(field_size(ArgumentKind::Trigger), 2);
        assert_eq!(field_size(ArgumentKind::Entity), 1);
        assert_eq!(field_size(ArgumentKind::Label), 2);
    }

    #[test]
    fn a_script_with_every_kind_of_instruction_disassembles() {
        let script = every_kind_of_instruction();
        let disassembly = disassemble_script(script.bytes()).unwrap();

        assert!(disassembly.unknowns.is_empty());
        assert_eq!(disassembly.code_end as usize, script.size());
        assert_eq!(disassembly.instructions.len(), 18);
        assert_eq!(
            disassembly.labels,
            vec![12, 66, 92, 94, 98, 103, 108]
        );
        // The SET_SCRIPT after the jump and the END_SCRIPT after the return.
        assert_eq!(disassembly.unreachable, 2);

        let set_pstat = instruction_at(&disassembly, 12);
        assert_eq!(opcode_name(set_pstat.opcode), "set_pstat");
        assert_eq!(set_pstat.size, 4);
        assert_eq!(set_pstat.arguments.len(), 2);
        assert_eq!(set_pstat.arguments[0].kind, ArgumentKind::Pstat);
        assert_eq!(set_pstat.arguments[1].value, 7);
        assert_eq!(set_pstat.pads.len(), 1);
        assert_eq!(set_pstat.pads[0].offset, 1);
        assert_eq!(set_pstat.pads[0].value, 0x11);

        assert_eq!(instruction_at(&disassembly, 16).arguments[0].value, 0x1234);
        assert_eq!(instruction_at(&disassembly, 20).arguments[0].value, -2);
        assert_eq!(instruction_at(&disassembly, 26).arguments[1].value, -200);

        let branch = instruction_at(&disassembly, 34);
        assert_eq!(branch.size, 12);
        assert_eq!(branch.conditions.len(), 2);
        assert_eq!(branch.conditions[0].test, ConditionTest::Pstat);
        assert_eq!(branch.conditions[0].combine, Combine::Replace);
        assert_eq!(branch.conditions[1].test, ConditionTest::GotoIfFalse);
        assert_eq!(branch.conditions[1].arguments[0].value, 92);

        assert_eq!(
            instruction_at(&disassembly, 46).text.as_deref(),
            Some("A")
        );
        assert_eq!(instruction_at(&disassembly, 58).targets, vec![66, 103]);

        let selection = instruction_at(&disassembly, 66);
        assert_eq!(selection.targets, vec![92, 98]);
        assert_eq!(selection.text.as_deref(), Some("A"));
        assert!(selection.selection_end_marker);
        assert_eq!(selection.size, 16);

        assert_eq!(instruction_at(&disassembly, 86).arguments[1].value, 4);
        assert_eq!(instruction_at(&disassembly, 98).data, vec![1, 2, 0xFF]);
        assert_eq!(instruction_at(&disassembly, 103).size, 5);
    }

    #[test]
    fn what_the_interpreter_would_not_run_is_an_unknown_not_a_guess() {
        assert_eq!(unknowns_of(&[0x40, 0x00, 0xFF, 0x00]).len(), 1); // no such opcode
        assert_eq!(unknowns_of(&[0x1E, 0x00, 0x05]).len(), 1); // operands past the end
        assert_eq!(
            unknowns_of(&[0x16, 0x00, 0x00, 0x02, 0xFF, 0x00]).len(),
            1
        ); // jump past the end: no END_SCRIPT after it
        assert_eq!(
            unknowns_of(&[0x16, 0x00, 0x09, 0x00, 0xFF, 0x00]).len(),
            1
        ); // jump into an instruction
        assert_eq!(unknowns_of(&[0xFE, 0x05, 0xFF, 0x00]).len(), 1); // odd start
        assert_eq!(
            unknowns_of(&[0x19, 0x00, 0x28, 0x00, 0x00, 0x00, 0x19, 0x00, 0xFF, 0x00]).len(),
            1
        ); // IF test
        assert_eq!(
            unknowns_of(&[0x19, 0x00, 0x0E, 0x00, 0x05, 0x01, 0x19, 0x00, 0xFF, 0x00]).len(),
            1
        ); // compare 6
        assert_eq!(
            unknowns_of(&[0x19, 0x00, 0x20, 0x00, 0x01, 0x07, 0x00, 0x00, 0x19, 0x00, 0xFF, 0x00]).len(),
            1
        );
        assert_eq!(unknowns_of(&[0x1A, 0x00, 0x82, 0x60]).len(), 1); // text that does not end
        assert_eq!(
            unknowns_of(&[0x10, 0x01, 0x08, 0x00, 0x00, 0x00, 0xFF, 0x00]).len(),
            1
        ); // no SHOW_TEXTBOX word
        assert_eq!(unknowns_of(&[0x6B, 0x00, 0x01, 0x02]).len(), 1); // DATA without its end
        assert!(unknowns_of(&[0xFE, 0x00, 0xFF, 0x00]).is_empty());

        let mut bad = ScriptBuilder::new(&[(0xFE, 8)]);
        bad.u8(&[0x40, 0x00]);
        let disassembly = disassemble_script(bad.bytes()).unwrap();
        assert_eq!(disassembly.code_end, 8);
        assert!(disassembly.instructions.is_empty());
        assert!(disassembly.unknowns[0].contains("0x40"));

        // A header whose code does not start after the list of sections.
        let mut bad_header = ScriptBuilder::new(&[(0xFE, 2)]);
        bad_header.u8(&[0xFF, 0x00]);
        assert!(disassemble_script(bad_header.bytes()).is_err());
    }
}
