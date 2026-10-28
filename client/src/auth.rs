


 

pub async fn login(username: String, password: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client.post("https://auth.kattmys.se/login")
        .json(&protocol::kattauth::LoginDetails {
            username,
            hashword: password,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Inloggningen misslyckades ({})", resp.status()));
    }

    resp.json::<String>()
        .await
        .map_err(|e| e.to_string())
}
