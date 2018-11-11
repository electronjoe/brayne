pub type UuidString = String;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum AttemptQuality {
    Perfect = 5,
    CorrectAfterHesitation = 4,
    CorrectSeriousDifficulty = 3,
    IncorrectButEasyRecall = 2,
    IncorrectButRemembered = 1,
    Blackout = 0,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AttemptRecord {
    pub uuid: UuidString,
    pub time: std::time::SystemTime,
    pub quality: AttemptQuality,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum CardContents {
    BasicCard(BasicCard),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BasicCard {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Card {
    pub uuid: UuidString,
    // Timestamp at time of card creation
    // Represents seconds of UTC time since Unix epoch
    pub created: std::time::SystemTime,
    // Tags are arbirary user-supplied strings, e.g. "travel"
    pub tags: Vec<String>,

    pub card_contents: CardContents,
}
