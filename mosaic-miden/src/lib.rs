pub mod note;
mod version;

use miden_assembly::{
    Assembler, DefaultSourceManager, Library, LibraryPath,
    ast::{Module, ModuleKind},
};
use miden_client::{
    ScriptBuilder,
    account::AccountId,
    note::{
        Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteTag,
        NoteType,
    },
    transaction::TransactionKernel,
};
use miden_lib::utils::Serializable;
use miden_objects::{Felt, Word};
use std::sync::Arc;

fn create_library(
    assembler: Assembler,
    modules: &[(String, String)],
) -> Result<Library, Box<dyn std::error::Error>> {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let mut parsed_modules = Vec::new();
    for (library_path, source_code) in modules {
        let lib_path = LibraryPath::new(library_path)?;
        let module = Module::parser(ModuleKind::Library).parse_str(
            lib_path,
            source_code,
            &source_manager,
        )?;
        parsed_modules.push(module);
    }
    let library = assembler.assemble_library(parsed_modules)?;
    Ok(library)
}

pub fn compile_note(
    note: note::MidenAbstractNote,
    account_id: AccountId,
    secret: Word,
    recipient: &String,
    inputs: note::Inputs,
) -> Result<note::MidenNote, Box<dyn std::error::Error>> {
    version::assert_version(&note.schema);
    let code = note.program;
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let note_script = if !&note.libraries.is_empty() {
        let libraries = create_library(assembler, &note.libraries)?;
        ScriptBuilder::new(true)
            .with_dynamically_linked_library(&libraries)?
            .compile_note_script(&code)?
    } else {
        ScriptBuilder::new(true).compile_note_script(&code)?
    };
    let mut inputs_inner: Vec<Felt> = vec![];
    for input in inputs {
        match input {
            (_, note::Value::Word(word)) => {
                for v in word {
                    inputs_inner.push(Felt::new(v));
                }
            }
            (_, note::Value::Element(element)) => inputs_inner.push(Felt::new(element)),
        }
    }
    let note_inputs = NoteInputs::new(inputs_inner)?;
    let note_recipient = NoteRecipient::new(secret, note_script, note_inputs);
    let tag = NoteTag::for_local_use_case(0, 0).unwrap();
    let metadata = NoteMetadata::new(
        account_id,
        NoteType::Private,
        tag,
        NoteExecutionHint::always(),
        Felt::new(0),
    )?;
    let assets = NoteAssets::new(vec![])?;
    let note_inner = Note::new(assets, metadata, note_recipient);
    let mut buffer = Vec::new();
    note_inner.write_into(&mut buffer);
    let note_inner_string = hex::encode(&buffer);

    let miden_note = note::MidenNote {
        schema: note.schema,
        note_type: note.note_type,
        recipient: recipient.clone(),
        miden_note_hex: note_inner_string,
    };
    Ok(miden_note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_note() {
        let test_account_id = AccountId::from_hex("0x1885b9f45e348800337a1a317a076b").unwrap();
        let note = note::MidenAbstractNote {
            schema: "MOSAIC 2025.10 MIDEN 0.11".to_string(),
            note_type: note::NoteType::Private,
            program: "begin nop end".to_string(),
            libraries: vec![],
        };
        let secret = Word::new([Felt::new(3); 4]);
        let recipient = "SomeOne".to_string();
        let miden_note = compile_note(note, test_account_id, secret, &recipient, vec![]).unwrap();
        let miden_note_json = serde_json::to_string(&miden_note).unwrap();
        println!("{}", miden_note_json);
        let miden_note: note::MidenNote = serde_json::from_str(&miden_note_json).unwrap();
        version::assert_version(&miden_note.schema);
        assert_eq!(miden_note.note_type, note::NoteType::Private);
        assert_eq!(miden_note.recipient, recipient);
        assert_eq!(
            miden_note.miden_note_hex,
            "0088345ef4b98518816b077a311a7a33000000c0000000000000000000000000004d41535400000000030303000000000503000000000000000030f0db3924f3e2d677a51924b09ecef8a12416a6ceb09fadd39785bb4f685cab6601011d010001010600000009000000030507010d272f0b24657865631924657865633a3a246d61696e076e6f7000000000000301030100000000010100000000000300000000000000030000000000000003000000000000000300000000000000"
        );
    }
}
