use crate::service_protocol::messages::{start_message::StateEntry, *};
use crate::service_protocol::Version;
use crate::tests::VMTestCase;
use crate::{CoreVM, NonEmptyValue, SuspendedOrVMError, Value, VM};
use assert2::let_assert;
use bytes::Bytes;

/// Normal state

fn get_state_handler(vm: &mut CoreVM) {
    vm.sys_input().unwrap();

    let h1 = vm.sys_get_state("STATE".to_owned()).unwrap();

    vm.notify_await_point(h1);
    let h1_result = vm.take_async_result(h1);
    if let Err(SuspendedOrVMError::Suspended(_)) = &h1_result {
        return;
    }

    let str_result = match h1_result.unwrap().unwrap() {
        Value::Void => "Unknown".to_owned(),
        Value::Success(s) => String::from_utf8(s).unwrap(),
        Value::Failure(f) => {
            vm.sys_write_output(NonEmptyValue::Failure(f)).unwrap();
            vm.sys_end().unwrap();
            return;
        }
    };

    vm.sys_write_output(NonEmptyValue::Success(str_result.into_bytes()))
        .unwrap();
    vm.sys_end().unwrap()
}

mod only_lazy_state {
    use super::*;

    use test_log::test;

    #[test]
    fn entry_already_completed() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 2,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco",
                ))),
                ..Default::default()
            })
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }
    #[test]
    fn entry_already_completed_empty() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 2,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            })
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Unknown"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }
    #[test]
    fn new_entry() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<SuspensionMessage>().unwrap(),
            SuspensionMessage {
                entry_indexes: vec![1],
            }
        );

        assert_eq!(output.next(), None);
    }
    #[test]
    fn entry_not_completed_on_replay() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 2,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            })
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<SuspensionMessage>().unwrap(),
            SuspensionMessage {
                entry_indexes: vec![1],
            }
        );

        assert_eq!(output.next(), None);
    }
    #[test]
    fn entry_on_replay_completed_later() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 2,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            })
            .input(CompletionMessage {
                entry_index: 1,
                result: Some(completion_message::Result::Value(Bytes::from_static(
                    b"Francesco",
                ))),
            })
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }
    #[test]
    fn new_entry_completed_later() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(CompletionMessage {
                entry_index: 1,
                result: Some(completion_message::Result::Value(Bytes::from_static(
                    b"Francesco",
                ))),
            })
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }
    #[test]
    fn replay_failed_get_state_entry() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 2,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Failure(Failure {
                    code: 409,
                    ..Default::default()
                })),
                ..Default::default()
            })
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Failure(Failure {
                    code: 409,
                    ..Default::default()
                })),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }
    #[test]
    fn complete_failing_get_state_entry() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(CompletionMessage {
                entry_index: 1,
                result: Some(completion_message::Result::Failure(Failure {
                    code: 409,
                    ..Default::default()
                })),
            })
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Failure(Failure {
                    code: 409,
                    ..Default::default()
                })),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }
}

/// Eager state

mod eager {
    use super::*;

    use test_log::test;

    fn get_empty_state_handler(vm: &mut CoreVM) {
        vm.sys_input().unwrap();

        let h1 = vm.sys_get_state("STATE".to_owned()).unwrap();

        vm.notify_await_point(h1);
        let h1_result = vm.take_async_result(h1);
        if let Err(SuspendedOrVMError::Suspended(_)) = &h1_result {
            return;
        }

        let str_result = match h1_result.unwrap().unwrap() {
            Value::Void => "true".to_owned(),
            Value::Success(_) => "false".to_owned(),
            Value::Failure(f) => {
                vm.sys_write_output(NonEmptyValue::Failure(f)).unwrap();
                vm.sys_end().unwrap();
                return;
            }
        };

        vm.sys_write_output(NonEmptyValue::Success(str_result.into_bytes()))
            .unwrap();
        vm.sys_end().unwrap()
    }

    #[test]
    fn get_empty_with_complete_state() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                ..Default::default()
            })
            .input(InputEntryMessage::default())
            .run(get_empty_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"true"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    #[test]
    fn get_empty_with_partial_state() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage::default())
            .run(get_empty_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<SuspensionMessage>().unwrap(),
            SuspensionMessage {
                entry_indexes: vec![1],
            }
        );

        assert_eq!(output.next(), None);
    }

    #[test]
    fn get_empty_resume_with_partial_state() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 2,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage::default())
            .input(GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            })
            .run(get_empty_state_handler);

        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"true"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    #[test]
    fn get_with_complete_state() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                state_map: vec![StateEntry {
                    key: Bytes::from_static(b"STATE"),
                    value: Bytes::from_static(b"Francesco"),
                }],
                key: "my-greeter".to_owned(),
                ..Default::default()
            })
            .input(InputEntryMessage::default())
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    #[test]
    fn get_with_partial_state() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                state_map: vec![StateEntry {
                    key: Bytes::from_static(b"STATE"),
                    value: Bytes::from_static(b"Francesco"),
                }],
                partial_state: true,
                key: "my-greeter".to_owned(),
            })
            .input(InputEntryMessage::default())
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    #[test]
    fn get_with_partial_state_without_the_state_entry() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage::default())
            .run(get_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<SuspensionMessage>().unwrap(),
            SuspensionMessage {
                entry_indexes: vec![1],
            }
        );
        assert_eq!(output.next(), None);
    }

    fn append_state_handler(vm: &mut CoreVM) {
        let input = vm.sys_input().unwrap().input;

        let h1 = vm.sys_get_state("STATE".to_owned()).unwrap();
        vm.notify_await_point(h1);
        let h1_result = vm.take_async_result(h1);
        if let Err(SuspendedOrVMError::Suspended(_)) = &h1_result {
            return;
        }
        let get_result = match h1_result.unwrap().unwrap() {
            Value::Void => {
                panic!("Unexpected empty get state")
            }
            Value::Success(s) => s,
            Value::Failure(f) => {
                vm.sys_write_output(NonEmptyValue::Failure(f)).unwrap();
                vm.sys_end().unwrap();
                return;
            }
        };

        vm.sys_set_state(
            "STATE".to_owned(),
            [get_result.clone(), input.clone()].concat(),
        )
        .unwrap();

        let h2 = vm.sys_get_state("STATE".to_owned()).unwrap();
        vm.notify_await_point(h2);
        let h2_result = vm.take_async_result(h2);
        if let Err(SuspendedOrVMError::Suspended(_)) = &h2_result {
            return;
        }
        let second_get_result = match h2_result.unwrap().unwrap() {
            Value::Void => {
                panic!("Unexpected empty get state")
            }
            Value::Success(s) => s,
            Value::Failure(f) => {
                vm.sys_write_output(NonEmptyValue::Failure(f)).unwrap();
                vm.sys_end().unwrap();
                return;
            }
        };

        vm.sys_write_output(NonEmptyValue::Success(second_get_result))
            .unwrap();
        vm.sys_end().unwrap()
    }

    #[test]
    fn append_with_state_in_the_state_map() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                state_map: vec![StateEntry {
                    key: Bytes::from_static(b"STATE"),
                    value: Bytes::from_static(b"Francesco"),
                }],
                partial_state: true,
                key: "my-greeter".to_owned(),
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .run(append_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<SetStateEntryMessage>().unwrap(),
            SetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                value: Bytes::from_static(b"FrancescoTill"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Value(Bytes::from_static(
                    b"FrancescoTill"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"FrancescoTill"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    #[test]
    fn append_with_partial_state_on_the_first_get() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(CompletionMessage {
                entry_index: 1,
                result: Some(completion_message::Result::Value(Bytes::from_static(
                    b"Francesco",
                ))),
            })
            .run(append_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<SetStateEntryMessage>().unwrap(),
            SetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                value: Bytes::from_static(b"FrancescoTill"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Value(Bytes::from_static(
                    b"FrancescoTill"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"FrancescoTill"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    fn get_and_clear_state_handler(vm: &mut CoreVM) {
        vm.sys_input().unwrap();

        let h1 = vm.sys_get_state("STATE".to_owned()).unwrap();
        vm.notify_await_point(h1);
        let h1_result = vm.take_async_result(h1);
        if let Err(SuspendedOrVMError::Suspended(_)) = &h1_result {
            return;
        }
        let first_get_result = match h1_result.unwrap().unwrap() {
            Value::Void => {
                panic!("Unexpected empty get state")
            }
            Value::Success(s) => s,
            Value::Failure(f) => {
                vm.sys_write_output(NonEmptyValue::Failure(f)).unwrap();
                vm.sys_end().unwrap();
                return;
            }
        };

        vm.sys_clear_state("STATE".to_owned()).unwrap();

        let h2 = vm.sys_get_state("STATE".to_owned()).unwrap();
        vm.notify_await_point(h2);
        let h2_result = vm.take_async_result(h2);
        if let Err(SuspendedOrVMError::Suspended(_)) = &h2_result {
            return;
        }
        let_assert!(Ok(Some(Value::Void)) = h2_result);

        vm.sys_write_output(NonEmptyValue::Success(first_get_result))
            .unwrap();
        vm.sys_end().unwrap()
    }

    #[test]
    fn get_and_clear_state_with_state_in_the_state_map() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                state_map: vec![StateEntry {
                    key: Bytes::from_static(b"STATE"),
                    value: Bytes::from_static(b"Francesco"),
                }],
                partial_state: true,
                key: "my-greeter".to_owned(),
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .run(get_and_clear_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<ClearStateEntryMessage>().unwrap(),
            ClearStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    #[test]
    fn get_and_clear_state_with_partial_state_on_the_first_get() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(CompletionMessage {
                entry_index: 1,
                result: Some(completion_message::Result::Value(Bytes::from_static(
                    b"Francesco",
                ))),
            })
            .run(get_and_clear_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<ClearStateEntryMessage>().unwrap(),
            ClearStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    fn get_and_clear_all_state_handler(vm: &mut CoreVM) {
        vm.sys_input().unwrap();

        let h1 = vm.sys_get_state("STATE".to_owned()).unwrap();
        vm.notify_await_point(h1);
        let h1_result = vm.take_async_result(h1);
        if let Err(SuspendedOrVMError::Suspended(_)) = &h1_result {
            return;
        }
        let first_get_result = match h1_result.unwrap().unwrap() {
            Value::Void => {
                panic!("Unexpected empty get state")
            }
            Value::Success(s) => s,
            Value::Failure(f) => {
                vm.sys_write_output(NonEmptyValue::Failure(f)).unwrap();
                vm.sys_end().unwrap();
                return;
            }
        };

        vm.sys_clear_all_state().unwrap();

        let h2 = vm.sys_get_state("STATE".to_owned()).unwrap();
        vm.notify_await_point(h2);
        let_assert!(Ok(Some(Value::Void)) = vm.take_async_result(h2));

        let h3 = vm.sys_get_state("ANOTHER_STATE".to_owned()).unwrap();
        vm.notify_await_point(h3);
        let_assert!(Ok(Some(Value::Void)) = vm.take_async_result(h3));

        vm.sys_write_output(NonEmptyValue::Success(first_get_result))
            .unwrap();
        vm.sys_end().unwrap()
    }

    #[test]
    fn get_clear_all_with_state_in_the_state_map() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                state_map: vec![
                    StateEntry {
                        key: Bytes::from_static(b"STATE"),
                        value: Bytes::from_static(b"Francesco"),
                    },
                    StateEntry {
                        key: Bytes::from_static(b"ANOTHER_STATE"),
                        value: Bytes::from_static(b"Francesco"),
                    },
                ],
                partial_state: true,
                key: "my-greeter".to_owned(),
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .run(get_and_clear_all_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<ClearAllStateEntryMessage>().unwrap(),
            ClearAllStateEntryMessage::default()
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"ANOTHER_STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    #[test]
    fn get_clear_all_with_partial_state_on_the_first_get() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                partial_state: true,
                ..Default::default()
            })
            .input(InputEntryMessage {
                value: Bytes::from_static(b"Till"),
                ..Default::default()
            })
            .input(CompletionMessage {
                entry_index: 1,
                result: Some(completion_message::Result::Value(Bytes::from_static(
                    b"Francesco",
                ))),
            })
            .run(get_and_clear_all_state_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<ClearAllStateEntryMessage>().unwrap(),
            ClearAllStateEntryMessage::default()
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"ANOTHER_STATE"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(
                    b"Francesco"
                ))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    fn consecutive_get_with_empty_handler(vm: &mut CoreVM) {
        vm.sys_input().unwrap();

        let h1 = vm.sys_get_state("key-0".to_owned()).unwrap();
        vm.notify_await_point(h1);
        let_assert!(Ok(Some(Value::Void)) = vm.take_async_result(h1));

        let h2 = vm.sys_get_state("key-0".to_owned()).unwrap();
        vm.notify_await_point(h2);
        let_assert!(Ok(Some(Value::Void)) = vm.take_async_result(h2));

        vm.sys_write_output(NonEmptyValue::Success(vec![])).unwrap();
        vm.sys_end().unwrap()
    }

    #[test]
    fn consecutive_get_with_empty() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 1,
                ..Default::default()
            })
            .input(InputEntryMessage::default())
            .run(consecutive_get_with_empty_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"key-0"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"key-0"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(b""))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }

    #[test]
    fn consecutive_get_with_empty_run_with_replay_of_the_first_get() {
        let mut output = VMTestCase::new(Version::V1)
            .input(StartMessage {
                id: Bytes::from_static(b"abc"),
                debug_id: "abc".to_owned(),
                known_entries: 2,
                ..Default::default()
            })
            .input(InputEntryMessage::default())
            .input(GetStateEntryMessage {
                key: Bytes::from_static(b"key-0"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            })
            .run(consecutive_get_with_empty_handler);

        assert_eq!(
            output.next_decoded::<GetStateEntryMessage>().unwrap(),
            GetStateEntryMessage {
                key: Bytes::from_static(b"key-0"),
                result: Some(get_state_entry_message::Result::Empty(Empty::default())),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<OutputEntryMessage>().unwrap(),
            OutputEntryMessage {
                result: Some(output_entry_message::Result::Value(Bytes::from_static(b""))),
                ..Default::default()
            }
        );
        assert_eq!(
            output.next_decoded::<EndMessage>().unwrap(),
            EndMessage::default()
        );
        assert_eq!(output.next(), None);
    }
}
