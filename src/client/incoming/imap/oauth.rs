pub struct OAuthCredentials {
    username: String,
    token: String,
}

impl async_imap::Authenticator for OAuthCredentials {
    type Response = String;

    fn process(&mut self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.username, self.token
        )
    }
}

impl OAuthCredentials {
    pub fn new<Username: Into<String>, Token: Into<String>>(
        username: Username,
        token: Token,
    ) -> Self {
        Self {
            username: username.into(),
            token: token.into(),
        }
    }
}
