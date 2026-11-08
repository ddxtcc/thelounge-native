use keyring::Entry;

const SERVICE: &str = "lounge-native";

fn account(server: &str, user: &str) -> Option<String> {
    let server = server.trim_end_matches('/');
    if server.is_empty() || user.is_empty() {
        return None;
    }
    Some(format!("{server}|{user}"))
}

pub(crate) fn load_token(server: &str, user: &str) -> Result<Option<String>, String> {
    let Some(account) = account(server, user) else {
        return Ok(None);
    };
    let entry = Entry::new(SERVICE, &account).map_err(|error| error.to_string())?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn store_token(server: &str, user: &str, token: &str) -> Result<(), String> {
    let Some(account) = account(server, user) else {
        return Ok(());
    };
    Entry::new(SERVICE, &account)
        .map_err(|error| error.to_string())?
        .set_password(token)
        .map_err(|error| error.to_string())
}

pub(crate) fn delete_token(server: &str, user: &str) -> Result<(), String> {
    let Some(account) = account(server, user) else {
        return Ok(());
    };
    let entry = Entry::new(SERVICE, &account).map_err(|error| error.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}
