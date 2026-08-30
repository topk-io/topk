use anyhow::Result;
use dialoguer::{Password, Select};

use crate::endpoint::Endpoint;

pub fn run(endpoint: &Endpoint) -> Result<Option<String>> {
    let choice = Select::new()
        .with_prompt("How would you like to authenticate with TopK?")
        .items(&["Create a new API key", "Use an existing API key", "Skip"])
        .default(0)
        .interact();

    match choice {
        // Open the console URL in the browser and prompt for the API key
        Ok(0) => {
            let scheme = if endpoint.https { "https" } else { "http" };
            let _ = open::that(format!("{scheme}://console.{}/api-key", endpoint.host));
            Ok(Some(prompt_api_key()?))
        }
        // Prompt for the API key directly
        Ok(1) => Ok(Some(prompt_api_key()?)),
        // Skip authentication
        Ok(_) => Ok(None),
        // Error
        Err(e) => Err(e.into()),
    }
}

fn prompt_api_key() -> Result<String> {
    let api_key = Password::new()
        .with_prompt("API key")
        .validate_with(|input: &String| {
            if input.trim().is_empty() {
                Err("API key cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact()?;

    Ok(api_key)
}
