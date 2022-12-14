//! Toplevel entrypoint command.

use crate::commands;
use abscissa_core::{
    command::Usage, Command, Config, Configurable, FrameworkError, Options, Runnable,
};
use diem_global_constants::NODE_HOME;
use std::path::PathBuf;

/// Toplevel entrypoint command.
///
/// Handles obtaining toplevel help as well as verbosity settings.
#[derive(Debug, Options)]
pub struct EntryPoint<Cmd>
where
    Cmd: Command + Runnable,
{
    /// Path to the configuration file
    #[options(short = "c", help = "path to configuration file")]
    pub config: Option<PathBuf>,

    /// Obtain help about the current command
    #[options(short = "h", help = "print help message")]
    pub help: bool,

    /// Increase verbosity setting
    #[options(short = "v", help = "be verbose")]
    pub verbose: bool,

    /// Subcommand to execute.
    ///
    /// The `command` option will delegate option parsing to the command type,
    /// starting at the first free argument.
    #[options(command)]
    pub command: Option<Cmd>,

    // --- Customizing EntryPoint --- //
    /// Swarm path - get tx params from swarm
    #[options(help = "swarm path to override tx params, testing only")]
    pub swarm_path: Option<PathBuf>,

    /// Swarm persona - what fixtures to use
    #[options(help = "use the fixtures of a persona, e.g. alice, eve")]
    pub swarm_persona: Option<String>,
}

impl<Cmd> EntryPoint<Cmd>
where
    Cmd: Command + Runnable,
{
    /// Borrow the underlying command type or print usage info and exit
    fn command(&self) -> &Cmd {
        self.command
            .as_ref()
            .unwrap_or_else(|| Cmd::print_usage_and_exit(&[]))
    }
}

impl<Cmd> Runnable for EntryPoint<Cmd>
where
    Cmd: Command + Runnable,
{
    fn run(&self) {
        self.command().run()
    }
}

impl<Cmd> Command for EntryPoint<Cmd>
where
    Cmd: Command + Runnable,
{
    /// Name of this program as a string
    fn name() -> &'static str {
        Cmd::name()
    }

    /// Description of this program
    fn description() -> &'static str {
        Cmd::description()
    }

    /// Version of this program
    fn version() -> &'static str {
        Cmd::version()
    }

    /// Authors of this program
    fn authors() -> &'static str {
        Cmd::authors()
    }

    /// Get usage information for a particular subcommand (if available)
    fn subcommand_usage(command: &str) -> Option<Usage> {
        Cmd::subcommand_usage(command)
    }
}

impl<Cfg, Cmd> Configurable<Cfg> for EntryPoint<Cmd>
where
    Cmd: Command + Configurable<Cfg> + Runnable,
    Cfg: Config,
{
    /// Path to the command's configuration file
    fn config_path(&self) -> Option<PathBuf> {
        match &self.config {
            // Use explicit `-c`/`--config` argument if passed
            Some(cfg) => Some(cfg.clone()),

            // Otherwise defer to the toplevel command's config path logic
            None => self.command.as_ref().and_then(|cmd| cmd.config_path()),
        }
    }

    /// Process the configuration after it has been loaded, potentially
    /// modifying it or returning an error if options are incompatible
    fn process_config(&self, config: Cfg) -> Result<Cfg, FrameworkError> {
        match &self.command {
            Some(cmd) => cmd.process_config(config),
            None => Ok(config),
        }
    }
}
/// the entry point args
pub type EntryPointTxsCmd = EntryPoint<commands::WizCmd>;
/// get arguments passed in the entrypoin of this app, not the subcommands
pub fn get_args() -> EntryPointTxsCmd {
    Command::from_env_args()
}

/// returns node_home
/// usually something like "/root/.0L"
/// in case of swarm like "....../swarm_temp/0" for alice
/// in case of swarm like "....../swarm_temp/1" for bob
pub fn get_node_home() -> PathBuf {
    let mut config_path = dirs::home_dir().unwrap();
    config_path.push(NODE_HOME);

    let entry_args = get_args();

    if entry_args.swarm_path.is_some() {
        config_path = PathBuf::from(entry_args.swarm_path.unwrap());
        if entry_args.swarm_persona.is_some() {
            let persona = &entry_args.swarm_persona.unwrap();
            let all_personas = vec!["alice", "bob", "carol", "dave"];
            let index = all_personas.iter().position(|&r| r == persona).unwrap();
            config_path.push(index.to_string());
        } else {
            config_path.push("0"); // default
        }
    }

    return config_path;
}
