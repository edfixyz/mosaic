use serde::{Deserialize, Serialize};

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
#[derive(PartialEq, Serialize, Deserialize, Debug)]
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
/// * `Public` - Note data is publicly visible
/// * `Private` - Note data is encrypted and private
#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub enum NoteType {
    /// Public note
    Public,
    /// Private note
    Private,
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
///     schema: "v1.0".to_string(),
///     note_type: NoteType::Private,
///     program: "begin push.1 end".to_string(),
///     libraries: vec![],
/// };
/// ```
#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub struct MidenAbstractNote {
    /// Schema version identifier
    pub schema: String,
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
///     schema: "v1.0".to_string(),
///     note_type: NoteType::Public,
///     recipient: "0x1234...".to_string(),
///     miden_note_hex: "a1b2c3...".to_string(),
/// };
/// ```
#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub struct MidenNote {
    /// Schema version identifier
    pub schema: String,
    /// Visibility type of the note
    pub note_type: NoteType,
    /// Address of the note recipient
    pub recipient: Recipient,
    /// Hexadecimal representation of the compiled note
    pub miden_note_hex: String,
}