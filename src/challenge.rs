use rand::{seq::SliceRandom, Rng};
use rust_persian_tools::{number_to_words::number_to_words, words_to_number::words_to_number};

pub struct Quiz {
    num1: u8,
    num2: u8,
}

impl Quiz {
    pub fn new() -> Self {
        let num1 = rand::thread_rng().gen_range(0..20);
        let num2 = rand::thread_rng().gen_range(0..20);

        Self { num1, num2 }
    }

    pub fn from_str(data: &str) -> Self {
        let nums = data.split(" + ").collect::<Vec<&str>>();
        Self {
            num1: words_to_number(nums[0]).unwrap_or_default() as _,
            num2: words_to_number(nums[1]).unwrap_or_default() as _,
        }
    }

    pub fn answer(&self) -> u16 {
        self.num1 as u16 + self.num2 as u16
    }

    pub fn choices(&self) -> Vec<String> {
        let mut rng = rand::thread_rng();

        let mut opts = (0..3)
            .map(|_| rng.gen_range(0..40).to_string())
            .collect::<Vec<String>>();
        opts.push(self.answer().to_string());
        opts.shuffle(&mut rng);

        opts
    }

    pub fn encode(&self) -> String {
        format!(
            "{} + {}",
            number_to_words(self.num1 as _).unwrap(),
            number_to_words(self.num2 as _).unwrap()
        )
    }
}
