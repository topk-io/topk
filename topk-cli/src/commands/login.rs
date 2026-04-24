use dialoguer::{Password, Select};
use topk_rs::Error;

fn console_url(https: bool, host: &str) -> String {
    let scheme = if https { "https" } else { "http" };
    format!("{}://console.{}/api-key", scheme, host)
}

pub fn run(host: &str, https: bool) -> Result<Option<String>, Error> {
    let choice = Select::new()
        .with_prompt("How would you like to authenticate with TopK?")
        .items(&["Create a new API key", "Use an existing API key", "Skip"])
        .default(0)
        .interact();

    match choice {
        // Open the console URL in the browser and prompt for the API key
        Ok(0) => {
            let _ = open::that(&console_url(https, host));
            Ok(Some(prompt_api_key()?))
        }
        // Prompt for the API key directly
        Ok(1) => Ok(Some(prompt_api_key()?)),
        // Skip authentication
        Ok(_) => Ok(None),
        // Error
        Err(e) => Err(Error::Input(anyhow::anyhow!(e.to_string()))),
    }
}

fn prompt_api_key() -> Result<String, Error> {
    let api_key = Password::new()
        .with_prompt("API key")
        .validate_with(|input: &String| {
            if input.trim().is_empty() {
                Err("API key cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact()
        .map_err(std::io::Error::other)?;

    Ok(api_key)
}
