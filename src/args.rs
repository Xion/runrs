//! Module handling command line arguments.

use std::env;
use std::ffi::OsString;
use std::iter::IntoIterator;
use std::path::PathBuf;

use clap::{self, AppSettings, Arg, ArgMatches, ArgSettings};
use conv::TryFrom;
use conv::errors::NoError;


/// Parse command line arguments and return matches' object.
#[inline]
pub fn parse() -> Result<Options, clap::Error> {
    parse_from_argv(env::args_os())
}

/// Parse application options from given array of arguments
/// (*all* arguments, including binary name).
#[inline]
pub fn parse_from_argv<I, T>(argv: I) -> Result<Options, clap::Error>
    where I: IntoIterator<Item=T>, T: Clone + Into<OsString>
{
    let parser = create_parser();
    let matches = try!(parser.get_matches_from_safe(argv));

    // Currently, clap can catch all errors related to arguments
    // so this always succeeds.
    let options = Options::try_from(matches).unwrap();
    Ok(options)
}


/// Structure that holds options received from the command line.
/// This includes the script to run and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Verbosity of the logging output.
    ///
    /// Corresponds to the number of times the -v flag has been passed.
    /// If -q has been used instead, this will be negative.
    pub verbosity: isize,
    /// Path to the script to run.
    pub script: PathBuf,
    /// Arguments to the script.
    pub args: Vec<String>,
}

impl<'a> TryFrom<ArgMatches<'a>> for Options {
    type Err = NoError;

    fn try_from(matches: ArgMatches<'a>) -> Result<Self, Self::Err> {
        let verbose_count = matches.occurrences_of(OPT_VERBOSE) as isize;
        let quiet_count = matches.occurrences_of(OPT_QUIET) as isize;
        let verbosity = verbose_count - quiet_count;

        // Script file will be provided, but it doesn't have to have arguments.
        // Assume an empty argument list if that's the case.
        let script = matches.value_of(ARG_SCRIPT).unwrap();
        let script_args = matches.values_of(ARG_SCRIPT_ARGV)
            .map(|argv| argv.map(|v| v.to_owned()).collect())
            .unwrap_or_else(|| vec![]);

        // TODO: build mode (debug or --release)

        Ok(Options{
            verbosity: verbosity,
            script: PathBuf::from(script),
            args: script_args,
        })
    }
}


custom_derive! {
    /// Enum describing the build mode for the script.
    #[derive(Clone, Debug, Eq, PartialEq,
             IterVariants(BuildModes))]
    pub enum BuildMode {
        /// Debug mode.
        Debug,
        /// Release mode (with optimizations). Equivalent of --release in Cargo.
        Release,
    }
}

impl Default for BuildMode {
    fn default() -> Self { BuildMode::Debug }
}


// Parser configuration

/// Type of the argument parser object
/// (which is called an "App" in clap's silly nomenclature).
type Parser<'p> = clap::App<'p, 'p>;


const APP_NAME: &'static str = "runrs";
const APP_DESC: &'static str = "Runner for Rust scripts";

const ARG_SCRIPT: &'static str = "script";
const ARG_SCRIPT_ARGV: &'static str = "argv";
const OPT_VERBOSE: &'static str = "verbose";
const OPT_QUIET: &'static str = "quiet";
const OPT_RELEASE: &'static str = "release";


/// Create the argument parser.
fn create_parser<'p>() -> Parser<'p> {
    let mut parser = Parser::new(APP_NAME);
    if let Some(version) = option_env!("CARGO_PKG_VERSION") {
        parser = parser.version(version);
    }
    parser
        .about(APP_DESC)

        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::ColorNever)

        // Verbosity flags.
        .arg(Arg::with_name(OPT_VERBOSE)
            .long("verbose").short("v")
            .set(ArgSettings::Multiple)
            .conflicts_with(OPT_QUIET)
            .help("Increase logging verbosity"))
        .arg(Arg::with_name(OPT_QUIET)
            .long("quiet").short("q")
            .set(ArgSettings::Multiple)
            .conflicts_with(OPT_VERBOSE)
            .help("Decrease logging verbosity"))

        // Script to run and its arguments.
        .arg(Arg::with_name(ARG_SCRIPT)
            .required(true)
            .help("Rust source file to build & execute")
            .value_name("FILE"))
        // This argument spec is capturing everything after the script path,
        // allowing for the arguments to be passed to the script itself.
        .arg(Arg::with_name(ARG_SCRIPT_ARGV)
            .required(false)
            .multiple(true)
            .use_delimiter(false)
            .help("Optional arguments passed to the compiled binary")
            .value_name("ARGS"))
        .setting(AppSettings::TrailingVarArg)

        .help_short("H")
        .version_short("V")
}
