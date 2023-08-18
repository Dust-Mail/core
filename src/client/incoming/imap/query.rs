pub struct QueryBuilder {
    query: Vec<String>,
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new().flags().size().uid()
    }
}

impl QueryBuilder {
    pub fn build(self) -> String {
        format!("({})", self.query.join(" "))
    }

    pub fn flags(mut self) -> Self {
        self.query.push(String::from("FLAGS"));

        self
    }

    pub fn size(mut self) -> Self {
        self.query.push(String::from("RFC822.SIZE"));

        self
    }

    pub fn body(mut self) -> Self {
        self.query.push(String::from("RFC822"));

        self
    }

    pub fn uid(mut self) -> Self {
        self.query.push(String::from("UID"));

        self
    }

    pub fn headers<H: Into<String>>(mut self, headers: Vec<H>) -> Self {
        if headers.len() != 0 {
            let headers: Vec<String> = headers.into_iter().map(|head| head.into()).collect();
            self.query
                .push(format!("BODY[HEADER.FIELDS ({})]", headers.join(" ")));
        } else {
            self.query.push(String::from("BODY[HEADER]"));
        }

        self
    }

    pub fn new() -> Self {
        Self { query: Vec::new() }
    }
}
