use crate::prover::{EthAddress, EthEvent, EthEventParams};
use ethabi::{ParamType, Token};
use hex::ToHex;
use near_sdk::AccountId;

/// Data that was emitted by the Ethereum Locked event.
#[derive(Debug, Eq, PartialEq)]
pub struct EthLockedEvent {
    pub locker_address: EthAddress,
    pub token: String,
    pub sender: String,
    pub token_id: String,
    pub recipient: AccountId,
    pub token_uri: String,
}

impl EthLockedEvent {
    fn event_params() -> EthEventParams {
        vec![
            ("token".to_string(), ParamType::Address, true),
            ("sender".to_string(), ParamType::Address, true),
            ("token_id".to_string(), ParamType::String, false),
            ("account_id".to_string(), ParamType::String, false),
            ("token_uri".to_string(), ParamType::String, false),
        ]
    }

    /// Parse raw log entry data.
    pub fn from_log_entry_data(data: &[u8]) -> Self {
        let event = EthEvent::from_log_entry_data("Locked", EthLockedEvent::event_params(), data);
        let token = event.log.params[0].value.clone().to_address().unwrap().0;
        let token = (&token).encode_hex::<String>();
        let sender = event.log.params[1].value.clone().to_address().unwrap().0;
        let sender = (&sender).encode_hex::<String>();

        let token_id = event.log.params[2].value.clone().to_string().unwrap();
        let recipient = event.log.params[3].value.clone().to_string().unwrap();
        let token_uri = event.log.params[4].value.clone().to_string().unwrap();

        Self {
            locker_address: event.locker_address,
            token,
            sender,
            token_id,
            recipient,
            token_uri,
        }
    }

    pub fn to_log_entry_data(&self) -> Vec<u8> {
        EthEvent::to_log_entry_data(
            "Locked",
            EthLockedEvent::event_params(),
            self.locker_address,
            vec![
                hex::decode(self.token.clone()).unwrap(),
                hex::decode(self.sender.clone()).unwrap(),
            ],
            vec![
                Token::String(self.token_id.to_string()),
                Token::String(self.recipient.clone()),
                Token::String(self.token_uri.clone()),
            ],
        )
    }
}
