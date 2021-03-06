use key::Key;
use std::str;
use events::Events;
// we process these
use events::ReceiveEvent;
// we emit these
use events::BossEvent::{GotMessage as B_GotMessage,
                        GotVerifier as B_GotVerifier, Happy as B_Happy,
                        Scared as B_Scared};
use events::SendEvent::GotVerifiedKey as S_GotVerifiedKey;

#[derive(Debug, PartialEq)]
enum State {
    S0_unknown_key,
    S1_unverified_key(Vec<u8>),
    S2_verified_key(Vec<u8>),
    S3_scared,
}

pub struct Receive {
    state: State,
}

impl Receive {
    pub fn new() -> Receive {
        Receive {
            state: State::S0_unknown_key,
        }
    }

    pub fn process(&mut self, event: ReceiveEvent) -> Events {
        use self::State::*;

        println!(
            "receive: current state = {:?}, got event = {:?}",
            self.state, event
        );

        let (newstate, actions) = match self.state {
            S0_unknown_key => self.do_S0_unknown_key(event),
            S1_unverified_key(ref key) => self.do_S1_unverified_key(key, event),
            S2_verified_key(ref key) => self.do_S2_verified_key(key, event),
            S3_scared => self.do_S3_scared(event),
        };

        self.state = newstate;
        actions
    }

    fn do_S0_unknown_key(&self, event: ReceiveEvent) -> (State, Events) {
        use events::ReceiveEvent::*;
        match event {
            GotMessage(side, phase, body) => panic!(),
            GotKey(key) => (State::S1_unverified_key(key), events![]),
        }
    }

    fn derive_key_and_decrypt(
        side: &str,
        key: &[u8],
        phase: &str,
        body: Vec<u8>,
    ) -> Option<Vec<u8>> {
        let data_key = Key::derive_phase_key(&side, &key, &phase);

        Key::decrypt_data(data_key.clone(), &body)
    }

    fn do_S1_unverified_key(
        &self,
        key: &[u8],
        event: ReceiveEvent,
    ) -> (State, Events) {
        use events::ReceiveEvent::*;
        match event {
            GotKey(_) => panic!(),
            GotMessage(side, phase, body) => {
                match Self::derive_key_and_decrypt(&side, &key, &phase, body) {
                    Some(plaintext) => {
                        // got_message_good
                        let msg =
                            Key::derive_key(&key, b"wormhole:verifier", 32); // TODO: replace 32 with KEY_SIZE const
                        (
                            State::S2_verified_key(key.to_vec()),
                            events![
                                S_GotVerifiedKey(key.to_vec()),
                                B_Happy,
                                B_GotVerifier(msg),
                                B_GotMessage(phase, plaintext)
                            ],
                        )
                    }
                    None => {
                        // got_message_bad
                        (State::S3_scared, events![B_Scared])
                    }
                }
            }
        }
    }

    fn do_S2_verified_key(
        &self,
        key: &[u8],
        event: ReceiveEvent,
    ) -> (State, Events) {
        use events::ReceiveEvent::*;
        match event {
            GotKey(_) => panic!(),
            GotMessage(side, phase, body) => {
                match Self::derive_key_and_decrypt(&side, &key, &phase, body) {
                    Some(plaintext) => {
                        // got_message_good
                        (
                            State::S2_verified_key(key.to_vec()),
                            events![B_GotMessage(phase, plaintext)],
                        )
                    }
                    None => {
                        // got_message_bad
                        (State::S3_scared, events![B_Scared])
                    }
                }
            }
        }
    }

    fn do_S3_scared(&self, event: ReceiveEvent) -> (State, Events) {
        use events::ReceiveEvent::*;
        match event {
            GotKey(_) => panic!(),
            GotMessage(_, _, _) => (State::S3_scared, events![]),
        }
    }
}
