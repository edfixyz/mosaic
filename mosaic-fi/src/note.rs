use miden_objects::Word;
use miden_objects::account::AccountId;
use mosaic_miden::note::{MidenAbstractNote, MidenNote, NoteType, Value};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Serialize, Deserialize, schemars::JsonSchema, Debug, Clone, Copy)]
pub enum Side {
    BUY,
    SELL,
}

pub type Market = String;
pub type UUID = u128;
pub type Amount = u64;
pub type Price = u64;

#[derive(PartialEq, Serialize, Deserialize, schemars::JsonSchema, Debug, Clone)]
pub enum Order {
    // Notes emited by Desk, consumed by Client
    KYCPassed {
        market: Market,
    },
    QuoteRequestOffer {
        market: Market,
        uuid: UUID,
        side: Side,
        amount: Amount,
        price: Price,
    },
    QuoteRequestNoOffer {
        market: Market,
        uuid: UUID,
    },
    LimitBuyOrderLocked,
    LimitBuyOrderNotLocked, // At that stage the order is firm
    LimitSellOrderLocked,
    LimitSellOrderNotLocked,

    // Notes emitted by Client, consumed by Desk
    QuoteRequest {
        market: Market,
        uuid: UUID,
        side: Side,
        amount: Amount,
    },
    LimitOrder {
        market: Market,
        uuid: UUID,
        side: Side,
        amount: Amount,
        price: Price,
    },

    // Notes emetted by Liqudity Providers, consumed by Desk
    LiquidityOffer {
        market: Market,
        uuid: UUID,
        amount: Amount,
        price: Price,
    },
}

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct MosaicNote {
    pub market: Market,
    pub order: Order,
    pub miden_note: MidenNote,
}

pub fn compile_note_from_account_id(
    account_id: AccountId,
    order: Order,
) -> Result<MosaicNote, Box<dyn std::error::Error>> {
    match order {
        Order::LiquidityOffer {
            ref market,
            uuid,
            amount,
            price,
        } => {
            let abs_note = MidenAbstractNote {
                version: mosaic_miden::version::VERSION_STRING.to_string(),
                note_type: NoteType::Private,
                program: include_str!("../masm/notes/lp_liquidity_offer.masm").to_string(),
                libraries: vec![(
                    "external_contract::book".to_string(),
                    include_str!("../masm/accounts/book.masm").to_string(),
                )],
            };
            let secret = Word::default();
            let uuid_high = (uuid >> 64) as u64;
            let uuid_low = uuid as u64;
            let inputs = vec![
                ("uuid".to_string(), Value::Word([uuid_high, uuid_low, 0, 0])),
                ("amount".to_string(), Value::Element(amount)),
                ("price".to_string(), Value::Element(price)),
            ];
            let miden_note: MidenNote =
                mosaic_miden::note::compile_note(abs_note, account_id, secret, inputs)?;

            Ok(MosaicNote {
                market: market.to_string(),
                order,
                miden_note,
            })
        }
        _ => todo!(),
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use miden_lib::testing::mock_account::MockAccountExt;
//     use miden_objects::testing::noop_auth_component::NoopAuthComponent;

//     // Mock Account for testing
//     fn create_mock_account() -> Account<()> {
//         let miden_account = miden_client::account::Account::mock(
//             12345u128,         // Simple account ID
//             NoopAuthComponent, // Simple no-op auth component
//         );
//         Account {
//             miden_account,
//             miden_client,
//         }
//     }

//     #[test]
//     fn test_compile_note_liquidity_offer() {
//         let account = create_mock_account();
//         let market = "BTC/USD".to_string();
//         let uuid = 123456789u128;
//         let amount = 1000u64;
//         let price = 50000u64;

//         let order = Order::LiquidityOffer {
//             market: market.clone(),
//             uuid,
//             amount,
//             price,
//         };
//         let result = compile_note(account, order);
//         assert!(
//             result.is_ok(),
//             "compile_note should succeed for LiquidityOffer"
//         );
//         let mosaic_note = result.unwrap();
//         assert_eq!(mosaic_note.market, market, "Market should match input");
//     }

//     #[test]
//     fn test_compile_note_uuid_splitting() {
//         // Test that UUID is correctly split into high and low parts
//         let account = create_mock_account();
//         let uuid: u128 = (u64::MAX as u128) << 64 | (u64::MAX as u128);

//         let order = Order::LiquidityOffer {
//             market: "ETH/USD".to_string(),
//             uuid,
//             amount: 500,
//             price: 3000,
//         };
//         let result = compile_note(account, order);
//         assert!(
//             result.is_ok(),
//             "compile_note should handle large UUID values"
//         );
//     }
// }
