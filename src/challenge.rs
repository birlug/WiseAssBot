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

        let mut opts: Vec<u16> = (0..40).filter(|&x| x != self.answer()).collect();
        opts.shuffle(&mut rng);

        let mut opts: Vec<String> = [&opts[..3], &[self.answer()]]
            .concat()
            .into_iter()
            .map(|x| x.to_string())
            .collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_quiz_choices_contains_answer() {
        let quiz = Quiz::new();
        assert!(quiz.choices().contains(&quiz.answer().to_string()));
    }

    #[test]
    fn test_quiz_choices_unique() {
        let quiz = Quiz::new();
        let unique = HashSet::<_>::from_iter(quiz.choices().into_iter());
        assert_eq!(unique.len(), 4);
    }
}
