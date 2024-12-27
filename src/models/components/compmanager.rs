//! Component Manager

use geekorm::Data;

/// Dependency Manager Enum
///
/// https://github.com/package-url/purl-spec/blob/master/PURL-TYPES.rst
#[derive(Data, Debug, Default, Clone, PartialEq, Eq, Hash)]
#[geekorm(from_string = "lowercase", to_string = "lowercase")]
pub enum ComponentManager {
    /// Alpine Linux
    #[geekorm(aliases = "apk,alpine")]
    Apk,
    /// Cargo / Rust
    #[geekorm(aliases = "cargo,rust,rustc,rustlang")]
    Cargo,
    /// Composer / PHP
    #[geekorm(aliases = "composer,php")]
    Composer,
    /// Debian / Ubuntu
    #[geekorm(aliases = "deb,debian")]
    Deb,
    /// Ruby Gem
    #[geekorm(aliases = "gem,ruby")]
    Gem,
    /// Generic
    #[geekorm(aliases = "generic")]
    Generic,
    /// NPM
    #[geekorm(aliases = "npm,node,javascript")]
    Npm,
    /// Go Modules
    #[geekorm(aliases = "go,golang")]
    Golang,
    /// Maven / Java / Kotlin
    #[geekorm(aliases = "maven,gradle,java,kotlin,jvm")]
    Maven,
    /// Python Pip
    #[geekorm(aliases = "pypi,pip,python")]
    PyPi,
    /// Nuget
    #[geekorm(aliases = "nuget,csharp")]
    Nuget,
    /// RPM (Redhat Package Manager)
    #[geekorm(aliases = "rpm,redhat")]
    Rpm,
    /// Unknown Package Manager
    #[default]
    Unknown,
}
