//! The YAML of a script, the way the reference writes it.
//!
//! The listing names every label, every operand and every byte the interpreter skips, so the file can be read back
//! and turned into the script it came from.

use crate::error::Result;
use crate::formats::script::disassembly::{
    opcode_name, Argument, ArgumentKind, Combine, Condition, ConditionTest, Disassembly, Instruction,
};
use crate::formats::script::scn_file::ScriptSlot;
use std::path::Path;

/// The name of a label: the offset of the instruction it marks.
pub fn label_name(offset: u32) -> String {
    format!("l_{:04x}", offset)
}

/// A hex number with its digits, the way the reference writes them.
fn hex(value: u32, digits: usize) -> String {
    format!("{:#0width$x}", value, width = digits + 2)
}

/// The names of the comparisons, in the order the interpreter uses.
const COMPARE_NAMES: [&str; 6] = ["eq", "ne", "ge", "le", "gt", "lt"];

/// The ids of the entities that are not NPCs.
const TAMER_ENTITY: i32 = 0xFD;
const PARTNER_ENTITY: i32 = 0xFC;

/// A node of the document: a scalar, a sequence or a mapping.
enum Node {
    Plain(String),
    Quoted(String),
    Seq(Vec<Node>),
    Map(Vec<(String, Node)>),
}

impl Node {
    fn number(value: i64) -> Node {
        Node::Plain(value.to_string())
    }

    fn boolean(value: bool) -> Node {
        Node::Plain(if value { "true" } else { "false" }.to_string())
    }
}

/// A text in double quotes, with what a double quoted scalar needs escaped.
fn quoted(text: &str) -> Node {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\x{:02x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
    Node::Quoted(out)
}

/// Writes the node, with its keys in the order they were set.
fn emit(node: &Node, indent: usize, out: &mut String) {
    match node {
        Node::Plain(text) | Node::Quoted(text) => out.push_str(text),
        Node::Seq(items) => {
            for item in items {
                out.push_str(&" ".repeat(indent));
                out.push('-');
                match item {
                    Node::Map(entries) => {
                        let mut first = true;
                        for (key, value) in entries {
                            if first {
                                out.push(' ');
                                first = false;
                            } else {
                                out.push_str(&" ".repeat(indent + 2));
                            }
                            out.push_str(key);
                            out.push(':');
                            emit_value(value, indent + 2, out);
                        }
                    }
                    item => {
                        out.push(' ');
                        emit(item, indent + 2, out);
                        out.push('\n');
                    }
                }
            }
        }
        Node::Map(entries) => {
            for (key, value) in entries {
                out.push_str(&" ".repeat(indent));
                out.push_str(key);
                out.push(':');
                emit_value(value, indent, out);
            }
        }
    }
}

/// Writes the value of a key: a scalar goes in the same line, the rest below. A sequence keeps the indent of its
/// key, the way the reference writes it.
fn emit_value(node: &Node, indent: usize, out: &mut String) {
    match node {
        Node::Plain(_) | Node::Quoted(_) => {
            out.push(' ');
            emit(node, indent, out);
            out.push('\n');
        }
        Node::Seq(_) => {
            out.push('\n');
            emit(node, indent, out);
        }
        Node::Map(_) => {
            out.push('\n');
            emit(node, indent + 2, out);
        }
    }
}

/// The value of an operand.
fn argument_node(argument: &Argument, is_condition: bool) -> Node {
    match argument.kind {
        ArgumentKind::Label => Node::Plain(label_name(argument.value as u32)),
        ArgumentKind::Entity => {
            if argument.value == TAMER_ENTITY {
                Node::Plain("tamer".to_string())
            } else if argument.value == PARTNER_ENTITY {
                Node::Plain("partner".to_string())
            } else {
                Node::number(i64::from(argument.value))
            }
        }
        _ => {
            if is_condition
                && argument.name == "compare"
                && argument.value >= 0
                && (argument.value as usize) < COMPARE_NAMES.len()
            {
                Node::Plain(COMPARE_NAMES[argument.value as usize].to_string())
            } else {
                Node::number(i64::from(argument.value))
            }
        }
    }
}

/// The name of a test, and the bits of the entry's code that hold it.
fn test_info(test: ConditionTest) -> (&'static str, u8, u8) {
    match test {
        ConditionTest::Trigger => ("trigger", 0x00, 0),
        ConditionTest::Pstat => ("pstat", 0x08, 0),
        ConditionTest::GotoIfTrue => ("goto_if_true", 0x10, 0),
        ConditionTest::GotoIfFalse => ("goto_if_false", 0x18, 0),
        ConditionTest::Stat => ("stat", 0x20, 0),
        ConditionTest::Card => ("card", 0x20, 1),
        ConditionTest::Move => ("move", 0x20, 2),
        ConditionTest::Condition => ("condition", 0x20, 3),
        ConditionTest::ItemCount => ("item_count", 0x20, 4),
        ConditionTest::Money => ("money", 0x20, 5),
    }
}

/// The mapping of a condition.
fn condition_node(condition: &Condition) -> Node {
    let (name, kind, low) = test_info(condition.test);
    let mut entries: Vec<(String, Node)> = vec![("test".to_string(), Node::Plain(name.to_string()))];

    let mut canonical = kind | low;
    let low = condition.code & 0x07;
    if condition.test == ConditionTest::Trigger && low != 0 {
        entries.push(("clear".to_string(), Node::boolean(true)));
        canonical |= 0x01;
    }
    if condition.test == ConditionTest::Pstat {
        let compare = COMPARE_NAMES
            .get(low as usize)
            .copied()
            .unwrap_or(COMPARE_NAMES[0]);
        entries.push(("compare".to_string(), Node::Plain(compare.to_string())));
        canonical |= low;
    }
    match condition.combine {
        Combine::And => {
            entries.push(("combine".to_string(), Node::Plain("and".to_string())));
            canonical |= 0x80;
        }
        Combine::Or => {
            entries.push(("combine".to_string(), Node::Plain("or".to_string())));
            canonical |= 0x40;
        }
        Combine::Replace => {}
    }
    for argument in &condition.arguments {
        entries.push((argument.name.clone(), argument_node(argument, true)));
    }
    if condition.code != canonical {
        entries.push(("code".to_string(), Node::Plain(hex(u32::from(condition.code), 2))));
    }

    Node::Map(entries)
}

/// The mapping of an instruction.
fn instruction_node(instruction: &Instruction, labelled: bool) -> Node {
    let mut entries: Vec<(String, Node)> = vec![("at".to_string(), Node::Plain(hex(instruction.offset, 4)))];
    if labelled {
        entries.push(("label".to_string(), Node::Plain(label_name(instruction.offset))));
    }
    entries.push((
        "op".to_string(),
        Node::Plain(opcode_name(instruction.opcode).to_string()),
    ));
    for argument in &instruction.arguments {
        entries.push((argument.name.clone(), argument_node(argument, false)));
    }
    if !instruction.conditions.is_empty() {
        entries.push((
            "conditions".to_string(),
            Node::Seq(instruction.conditions.iter().map(condition_node).collect()),
        ));
    }
    if !instruction.targets.is_empty() {
        let key = if instruction.text.is_some() {
            "options"
        } else {
            "cases"
        };
        entries.push((
            key.to_string(),
            Node::Seq(
                instruction
                    .targets
                    .iter()
                    .map(|target| Node::Plain(label_name(u32::from(*target))))
                    .collect(),
            ),
        ));
    }
    if let Some(text) = &instruction.text {
        entries.push(("text".to_string(), quoted(text)));
    }
    if instruction.selection_end_marker {
        entries.push(("cancel_marker".to_string(), Node::boolean(true)));
    }
    if !instruction.data.is_empty() {
        let bytes: Vec<String> = instruction.data.iter().map(|byte| format!("{:02x}", byte)).collect();
        entries.push(("data".to_string(), quoted(&bytes.join(" "))));
    }
    if !instruction.pads.is_empty() {
        entries.push((
            "pads".to_string(),
            Node::Seq(
                instruction
                    .pads
                    .iter()
                    .map(|pad| {
                        Node::Map(vec![
                            ("offset".to_string(), Node::number(i64::from(pad.offset))),
                            ("value".to_string(), Node::Plain(hex(u32::from(pad.value), 2))),
                        ])
                    })
                    .collect(),
            ),
        ));
    }

    Node::Map(entries)
}

/// The YAML of a script.
pub fn script_to_yaml(disassembly: &Disassembly) -> String {
    let mut entries: Vec<(String, Node)> = vec![
        (
            "code_start".to_string(),
            Node::Plain(hex(u32::from(disassembly.header.code_start), 4)),
        ),
        ("code_end".to_string(), Node::Plain(hex(disassembly.code_end, 4))),
    ];

    let sections: Vec<Node> = disassembly
        .header
        .sections
        .iter()
        .map(|section| {
            Node::Map(vec![
                ("id".to_string(), Node::Plain(hex(u32::from(section.id), 4))),
                (
                    "at".to_string(),
                    Node::Plain(label_name(u32::from(section.offset))),
                ),
            ])
        })
        .collect();
    entries.push(("sections".to_string(), Node::Seq(sections)));

    entries.push((
        "unreachable_instructions".to_string(),
        Node::number(disassembly.unreachable as i64),
    ));

    if !disassembly.unknowns.is_empty() {
        entries.push((
            "unknowns".to_string(),
            Node::Seq(disassembly.unknowns.iter().map(|unknown| quoted(unknown)).collect()),
        ));
    }

    let instructions: Vec<Node> = disassembly
        .instructions
        .iter()
        .map(|instruction| {
            let labelled = disassembly
                .labels
                .binary_search(&u16::try_from(instruction.offset).unwrap_or(u16::MAX))
                .is_ok();
            instruction_node(instruction, labelled)
        })
        .collect();
    entries.push(("instructions".to_string(), Node::Seq(instructions)));

    let mut out = String::new();
    emit(&Node::Map(entries), 0, &mut out);
    out
}

/// Writes the YAML of a script.
pub fn write_script_yaml<P: AsRef<Path>>(path: P, disassembly: &Disassembly) -> Result<()> {
    std::fs::write(path, script_to_yaml(disassembly))?;
    Ok(())
}

/// The YAML of every script of a file of scripts, one document each.
pub fn scn_file_to_yaml(data: &[u8]) -> Result<Vec<(ScriptSlot, String)>> {
    let slots = crate::formats::script::scn_file::read_script_table(data)?;
    let mut documents = Vec::with_capacity(slots.len());
    for slot in &slots {
        let script = &data[slot.offset as usize..(slot.offset + slot.size) as usize];
        let disassembly = crate::formats::script::disassemble_script(script)?;
        documents.push((*slot, script_to_yaml(&disassembly)));
    }
    Ok(documents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::script::disassembly::disassemble_script;
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
        script.u8(&[0x19, 0x00, 0x0A, 0x00, 5, 2, 0x18, 0x00]); // 34 if pstat 5 >= 2
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

    #[test]
    fn the_listing_names_its_labels_and_keeps_every_byte_meaning() {
        let disassembly = disassemble_script(every_kind_of_instruction().bytes()).unwrap();
        let text = script_to_yaml(&disassembly);

        for expected in [
            "code_start: 0x000c",
            "- id: 0x00fe\n  at: l_000c",
            "unreachable_instructions: 2",
            "label: l_005c",
            "op: set_pstat",
            "pads:\n  - offset: 1\n    value: 0x11",
            "entity: partner",
            "rotation: -200",
            "amount: -2",
            "- test: pstat\n    compare: ge\n    pstat: 5\n    value: 2",
            "target: l_005c",
            "cases:\n  - l_0042\n  - l_0067",
            "options:\n  - l_005c\n  - l_0062",
            "text: \"A\"",
            "cancel_marker: true",
            "data: \"01 02 ff\"",
            "op: set_script\n  script: 3\n  map: 4",
            "op: end_script",
        ] {
            assert!(text.contains(expected), "the listing does not have:\n{}", expected);
        }
        assert!(!text.contains("unknowns"));
    }
}
