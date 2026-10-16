use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{BackendResult, CommandHandler, CommandsBackend};

use super::bridge::Bridge;
use super::commands::{CommandsPort, is_registered, listing, run_mod_command};

fn port() -> (Rc<Bridge>, CommandsPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), CommandsPort(bridge))
}

fn echo() -> CommandHandler {
    Box::new(|args: &[&str]| -> BackendResult<String> { Ok(args.join(" ")) })
}

fn failing() -> CommandHandler {
    Box::new(|_: &[&str]| -> BackendResult<String> { Err(BB_ERR_ARG) })
}

#[test]
fn a_registered_command_runs_its_handler_with_the_words_given() {
    let (bridge, port) = port();
    let handle = port.register("example.a", "Say", "says its words", echo()).unwrap();
    assert_ne!(handle, 0, "0 means none");
    assert!(is_registered(&bridge, "say"), "the console lower-cases the name it reads");
    assert_eq!(run_mod_command(&bridge, "say", &["hi", "there"]), Ok("hi there".to_owned()));
}

#[test]
fn names_are_plain_and_unique_across_mods() {
    let (_, port) = port();
    assert_eq!(port.register("example.a", "", "", echo()), Err(BB_ERR_ARG));
    assert_eq!(port.register("example.a", "two words", "", echo()), Err(BB_ERR_ARG));
    port.register("example.a", "say", "", echo()).unwrap();
    assert_eq!(port.register("example.b", "SAY", "", echo()), Err(BB_ERR_EXISTS));
}

#[test]
fn a_mod_removes_only_its_own_commands() {
    let (bridge, port) = port();
    let handle = port.register("example.a", "say", "", echo()).unwrap();
    assert_eq!(port.unregister("example.b", handle), Err(BB_ERR_NOT_FOUND));
    port.unregister("example.a", handle).unwrap();
    assert!(!is_registered(&bridge, "say"));
    assert_eq!(port.unregister("example.a", handle), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn run_queues_the_line_for_the_console_and_returns_no_output() {
    let (bridge, port) = port();
    assert_eq!(port.run("example.a", "  set fps 30 "), Ok(String::new()));
    assert_eq!(*bridge.queued_lines.borrow(), vec!["set fps 30".to_owned()]);
    assert_eq!(port.run("example.a", "   "), Err(BB_ERR_ARG));
    assert_eq!(bridge.queued_lines.borrow().len(), 1, "an empty line queues nothing");
}

#[test]
fn help_lists_each_mod_command_with_its_owner() {
    let (bridge, port) = port();
    port.register("example.a", "say", "says its words", echo()).unwrap();
    assert_eq!(listing(&bridge), vec![("say".to_owned(), "says its words (mod example.a)".to_owned())]);
}

#[test]
fn an_unknown_or_failing_command_is_an_error() {
    let (bridge, port) = port();
    assert!(run_mod_command(&bridge, "nope", &[]).unwrap_err().contains("unknown command"));
    port.register("example.a", "fail", "", failing()).unwrap();
    assert!(run_mod_command(&bridge, "fail", &[]).unwrap_err().contains("failed"));
}

#[test]
fn a_command_may_register_another_while_it_runs() {
    let (bridge, port) = port();
    let inner = bridge.clone();
    let first: CommandHandler = Box::new(move |_: &[&str]| -> BackendResult<String> {
        CommandsPort(inner.clone()).register("example.a", "later", "", echo())?;
        Ok(String::new())
    });
    port.register("example.a", "first", "", first).unwrap();
    assert_eq!(run_mod_command(&bridge, "first", &[]), Ok(String::new()));
    assert!(is_registered(&bridge, "later"), "the table was free while the handler ran");
}
