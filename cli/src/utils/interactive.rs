#![allow(dead_code)]

use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};

/// Prompt for input
#[allow(dead_code)]
pub fn prompt_input(name: &str) -> Result<String> {
    let input = dialoguer::Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(name)
        .interact_text()?;

    Ok(input)
}

pub fn prompt_password(name: &str) -> Result<String> {
    let password = dialoguer::Password::with_theme(&ColorfulTheme::default())
        .with_prompt(name)
        .interact()?;

    Ok(password)
}

pub fn prompt_confirm(name: &str) -> Result<bool> {
    let confirm = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(name)
        .interact()?;

    Ok(confirm)
}

pub fn prompt_select<'a>(name: &'a str, items: &Vec<&'a str>) -> Result<(&'a str, usize)> {
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(name)
        .default(0)
        .items(items)
        .interact()?;

    let text = items.get(selection).ok_or(anyhow!("No item selected"))?;

    Ok((text, selection))
}

pub fn prompt_select_with_default<'a>(
    name: &'a str,
    items: &Vec<&'a str>,
    default: usize,
) -> Result<(&'a str, usize)> {
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(name)
        .default(default)
        .items(items)
        .interact()?;

    let text = items.get(selection).ok_or(anyhow!("No item selected"))?;

    Ok((text, selection))
}
