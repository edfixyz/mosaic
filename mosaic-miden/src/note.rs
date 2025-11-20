use crate::{MidenTransactionId, version};

use miden_assembly::{
    Assembler, DefaultSourceManager, Library, LibraryPath,
    ast::{Module, ModuleKind},
};
use miden_client::{Client, keystore::FilesystemKeyStore};
use miden_client::{
    ScriptBuilder,
    account::AccountId,
    note::{Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteTag},
    transaction::{OutputNote, TransactionKernel, TransactionRequestBuilder},
};
use miden_lib::{
    note::create_p2id_note,
    utils::{Deserializable, Serializable},
};
use miden_objects::{
    Felt, Word, asset::FungibleAsset, crypto::rand::RpoRandomCoin, note::NoteScript,
    note::NoteType as MidenNoteType, transaction::TransactionScript,
};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Source code of a Miden program.
pub type ProgramSource = String;

/// Address or identifier of a note recipient.
pub type Recipient = String;

/// Represents a value that can be stored in a Miden note.
///
/// # Variants
///
/// * `Word` - A 256-bit word represented as four 64-bit unsigned integers
/// * `Element` - A single 64-bit field element
#[derive(PartialEq, Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub enum Value {
    /// A 256-bit word (4 × 64-bit elements)
    Word([u64; 4]),
    /// A single field element
    Element(u64),
}

/// A single input consisting of a name and its associated value.
///
/// The tuple format is `(input_name, input_value)`.
pub type Input = (String, Value);

/// A collection of inputs for a Miden note.
pub type Inputs = Vec<Input>;

/// Specifies the visibility type of a Miden note.
///
/// # Variants
///
/// * `Public` - Note data is publicly visible onchain
/// * `Private` - Note is private
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy, schemars::JsonSchema)]
pub enum NoteType {
    /// Public note
    Public,
    /// Private note
    Private,
}

impl From<NoteType> for miden_client::note::NoteType {
    fn from(note_type: NoteType) -> Self {
        match note_type {
            NoteType::Public => miden_client::note::NoteType::Public,
            NoteType::Private => miden_client::note::NoteType::Private,
        }
    }
}

impl From<miden_client::note::NoteType> for NoteType {
    fn from(note_type: miden_client::note::NoteType) -> Self {
        match note_type {
            miden_client::note::NoteType::Public => NoteType::Public,
            miden_client::note::NoteType::Private => NoteType::Private,
            miden_client::note::NoteType::Encrypted => todo!(),
        }
    }
}

/// An abstract representation of a Miden note before compilation.
///
/// This structure contains the high-level definition of a note including
/// its schema, visibility type, program logic, and any required libraries.
///
/// # Examples
///
/// ```
/// use mosaic_miden::note::{MidenAbstractNote, NoteType};
///
/// let note = MidenAbstractNote {
///     version: "MOSAIC 2025.10 MIDEN 0.11".to_string(),
///     note_type: NoteType::Private,
///     program: "begin push.1 drop end".to_string(),
///     libraries: vec![],
/// };
/// ```
#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub struct MidenAbstractNote {
    /// Schema version identifier
    pub version: String,
    /// Visibility type of the note
    pub note_type: NoteType,
    /// Miden assembly source code for the note's program
    pub program: ProgramSource,
    /// External libraries as (name, source) pairs
    pub libraries: Vec<(String, ProgramSource)>,
}

/// A compiled Miden note ready for use on the network.
///
/// This represents the final form of a note after compilation, containing
/// the recipient address and the serialized note data.
///
/// # Examples
///
/// ```
/// use mosaic_miden::note::{MidenNote, NoteType};
///
/// let note = MidenNote {
///     version: "MOSAIC 2025.10 MIDEN 0.11".to_string(),
///     note_type: NoteType::Public,
///     miden_note_hex: "a1b2c3...".to_string(),
/// };
/// ```
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct MidenNote {
    /// Schema version identifier
    pub version: String,
    /// Visibility type of the note
    pub note_type: NoteType,
    /// Hexadecimal representation of the compiled note
    pub miden_note_hex: String,
}

fn create_library(
    assembler: Assembler,
    modules: &[(String, ProgramSource)],
) -> Result<Library, Box<dyn std::error::Error>> {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let mut parsed_modules = Vec::new();
    for (library_path, source_code) in modules {
        let lib_path = LibraryPath::new(library_path.as_str())?;
        let module = Module::parser(ModuleKind::Library).parse_str(
            lib_path,
            source_code.as_str(),
            &source_manager,
        )?;
        parsed_modules.push(module);
    }
    let library = assembler.assemble_library(parsed_modules)?;
    Ok(library)
}

pub fn build_note_script(
    note: &MidenAbstractNote,
) -> Result<NoteScript, Box<dyn std::error::Error>> {
    version::assert_version(&note.version);
    let code = &note.program;
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let note_script = if !&note.libraries.is_empty() {
        let libraries = create_library(assembler, &note.libraries)?;
        ScriptBuilder::new(true)
            .with_dynamically_linked_library(&libraries)?
            .compile_note_script(code.as_str())?
    } else {
        ScriptBuilder::new(true).compile_note_script(code.as_str())?
    };
    Ok(note_script)
}

pub fn build_tx_script(
    note: &MidenAbstractNote,
) -> Result<TransactionScript, Box<dyn std::error::Error>> {
    version::assert_version(&note.version);
    let code = &note.program;
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let tx_script = if !&note.libraries.is_empty() {
        let libraries = create_library(assembler, &note.libraries)?;
        ScriptBuilder::new(true)
            .with_dynamically_linked_library(&libraries)?
            .compile_tx_script(code.as_str())?
    } else {
        ScriptBuilder::new(true).compile_tx_script(code.as_str())?
    };
    Ok(tx_script)
}

/// Compile an abstract note
///
pub fn compile_note(
    note: MidenAbstractNote,
    account_id: AccountId,
    secret: Word,
    inputs: Inputs,
) -> Result<MidenNote, Box<dyn std::error::Error>> {
    let note_script = build_note_script(&note).unwrap();
    let mut inputs_inner: Vec<Felt> = vec![];
    for input in inputs {
        match input {
            (_, Value::Word(word)) => {
                for v in word {
                    inputs_inner.push(Felt::new(v));
                }
            }
            (_, Value::Element(element)) => inputs_inner.push(Felt::new(element)),
        }
    }
    let note_inputs = NoteInputs::new(inputs_inner)?;
    let note_recipient = NoteRecipient::new(secret, note_script, note_inputs);
    let tag = NoteTag::for_local_use_case(0, 0).unwrap();
    let metadata = NoteMetadata::new(
        account_id,
        miden_client::note::NoteType::Private,
        tag,
        NoteExecutionHint::always(),
        Felt::new(0),
    )?;
    let assets = NoteAssets::new(vec![])?;
    let note_inner = Note::new(assets, metadata, note_recipient);

    let mut buffer = Vec::new();
    note_inner.write_into(&mut buffer);
    let note_inner_string = hex::encode(&buffer);

    let miden_note = MidenNote {
        version: note.version,
        note_type: note.note_type,
        miden_note_hex: note_inner_string,
    };
    Ok(miden_note)
}

/// Compile a P2ID (Pay-to-ID) note for transferring fungible assets
///
/// This function creates a note that transfers fungible assets from a faucet account
/// to a target account using Miden's built-in P2ID note functionality.
///
/// # Arguments
///
/// * `faucet_account_id` - The account ID of the faucet sending the assets
/// * `target_account_id` - The account ID of the recipient
/// * `amount` - The amount of tokens to transfer
/// * `rng` - Random number generator for note creation
///
/// # Returns
///
/// Returns a `MidenNote` containing the serialized P2ID note
pub fn compile_p2id_note(
    faucet_account_id: AccountId,
    target_account_id: AccountId,
    amount: u64,
    rng: &mut RpoRandomCoin,
) -> Result<MidenNote, Box<dyn std::error::Error>> {
    // Create a fungible asset from the faucet
    let fungible_asset = FungibleAsset::new(faucet_account_id, amount)?;

    // Create a P2ID note to send the asset to the target account
    let p2id_note = create_p2id_note(
        faucet_account_id,
        target_account_id,
        vec![fungible_asset.into()],
        MidenNoteType::Private,
        Felt::new(0),
        rng,
    )?;

    // Serialize the note
    let mut buffer = Vec::new();
    p2id_note.write_into(&mut buffer);
    let note_hex = hex::encode(&buffer);

    Ok(MidenNote {
        version: version::VERSION_STRING.to_string(),
        note_type: NoteType::Private,
        miden_note_hex: note_hex,
    })
}

pub async fn commit_note(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    account_id: AccountId,
    note: &MidenNote,
) -> Result<MidenTransactionId, Box<dyn std::error::Error>> {
    // Decode note hex
    let note_bytes = match hex::decode(&note.miden_note_hex) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(
                error = %e,
                account_id = %account_id,
                note_version = %note.version,
                note_type = ?note.note_type,
                note_hex = %note.miden_note_hex,
                note_hex_length = note.miden_note_hex.len(),
                "Failed to decode note hex"
            );
            return Err(Box::new(e));
        }
    };

    // Deserialize note
    let note_inner = match Note::read_from_bytes(&note_bytes) {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(
                error = %e,
                account_id = %account_id,
                note_version = %note.version,
                note_type = ?note.note_type,
                note_hex = %note.miden_note_hex,
                note_bytes_length = note_bytes.len(),
                note_bytes_hex = %hex::encode(&note_bytes),
                "Failed to deserialize note from bytes"
            );
            return Err(Box::new(e));
        }
    };

    // Build transaction request
    let tx_req = match TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(note_inner.clone())])
        .build()
    {
        Ok(req) => req,
        Err(e) => {
            tracing::error!(
                error = %e,
                account_id = %account_id,
                note_version = %note.version,
                note_type = ?note.note_type,
                note_id = %note_inner.id(),
                note_metadata = ?note_inner.metadata(),
                note_assets = ?note_inner.assets(),
                "Failed to build transaction request"
            );
            return Err(Box::new(e));
        }
    };

    // Execute transaction
    let tx_id = match client.submit_new_transaction(account_id, tx_req).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(
                error = %e,
                error_debug = ?e,
                account_id = %account_id,
                note_version = %note.version,
                note_type = ?note.note_type,
                note_id = %note_inner.id(),
                note_metadata = ?note_inner.metadata(),
                note_assets = ?note_inner.assets(),
                note_recipient = ?note_inner.recipient(),
                "Failed to execute transaction"
            );
            return Err(Box::new(e));
        }
    };

    tracing::info!(
        transaction_id = %tx_id,
        account_id = %account_id,
        "Transaction executed"
    );

    Ok(tx_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_note() {
        let test_account_id = AccountId::from_hex("0x1885b9f45e348800337a1a317a076b").unwrap();
        let note = MidenAbstractNote {
            version: "MOSAIC 2025.10 MIDEN 0.11".to_string(),
            note_type: NoteType::Private,
            program: "begin nop end".to_string(),
            libraries: vec![],
        };
        let secret = Word::new([Felt::new(3); 4]);
        let miden_note = compile_note(note, test_account_id, secret, vec![]).unwrap();
        let miden_note_json = serde_json::to_string(&miden_note).unwrap();
        let miden_note: MidenNote = serde_json::from_str(&miden_note_json).unwrap();
        version::assert_version(&miden_note.version);
        assert_eq!(miden_note.note_type, NoteType::Private);
        assert_eq!(
            miden_note.miden_note_hex,
            "0088345ef4b98518816b077a311a7a33000000c0000000000000000000000000004d41535400000000030303000000000503000000000000000030f0db3924f3e2d677a51924b09ecef8a12416a6ceb09fadd39785bb4f685cab6601011d010001010600000009000000030507010d272f0b24657865631924657865633a3a246d61696e076e6f7000000000000301030100000000010100000000000300000000000000030000000000000003000000000000000300000000000000"
        );
    }
}
