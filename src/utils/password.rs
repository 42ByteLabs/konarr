//! Password utilities

/// Password strength levels
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PasswordStrength {
    /// Very Weak Password
    VeryWeak = 0,
    /// Weak Password
    Weak = 1,
    /// Moderate Password
    Moderate = 2,
    /// Strong Password
    Strong = 3,
    /// Very Strong Password
    VeryStrong = 4,
}

/// Password strength validation
///
/// Returns a number indicating the strength of the password:
/// - 0: Very Weak
/// - 1: Weak
/// - 2: Moderate
/// - 3: Strong
/// - 4: Very Strong
pub fn validate_password_strength(password: &str) -> PasswordStrength {
    // Check if password is in the common insecure list
    let lowercase_password = password.to_lowercase();
    if insecure_list().contains(&lowercase_password.as_str()) {
        return PasswordStrength::VeryWeak; // Immediately return Very Weak for common passwords
    }

    let mut score = 0u8;

    // Length criteria
    let length = password.len();
    if length >= 8 {
        score += 1;
    }
    if length >= 12 {
        score += 1;
    }
    if length >= 16 {
        score += 1;
    }

    // Character complexity criteria
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    // Award points for character complexity
    let mut complexity_count = 0;
    if has_lowercase {
        complexity_count += 1;
    }
    if has_uppercase {
        complexity_count += 1;
    }
    if has_digit {
        complexity_count += 1;
    }
    if has_special {
        complexity_count += 1;
    }

    // Award score based on complexity (max 2 points)
    if complexity_count >= 2 {
        score += 1;
    }
    if complexity_count >= 3 {
        score += 1;
    }
    score.min(4).into()
}

/// List of the top 50 most common insecure passwords
/// These passwords should never be used as they are easily guessable
fn insecure_list() -> Vec<&'static str> {
    vec![
        "123456",
        "password",
        "12345678",
        "qwerty",
        "123456789",
        "12345",
        "1234",
        "111111",
        "1234567",
        "dragon",
        "123123",
        "baseball",
        "abc123",
        "football",
        "monkey",
        "letmein",
        "shadow",
        "master",
        "666666",
        "qwertyuiop",
        "123321",
        "mustang",
        "1234567890",
        "michael",
        "654321",
        "superman",
        "1qaz2wsx",
        "7777777",
        "121212",
        "000000",
        "qazwsx",
        "123qwe",
        "killer",
        "trustno1",
        "jordan",
        "jennifer",
        "zxcvbnm",
        "asdfgh",
        "hunter",
        "buster",
        "soccer",
        "harley",
        "batman",
        "andrew",
        "tigger",
        "sunshine",
        "iloveyou",
        "2000",
        "charlie",
        "robert",
    ]
}

impl From<u8> for PasswordStrength {
    fn from(value: u8) -> Self {
        match value {
            0 => PasswordStrength::VeryWeak,
            1 => PasswordStrength::Weak,
            2 => PasswordStrength::Moderate,
            3 => PasswordStrength::Strong,
            4 => PasswordStrength::VeryStrong,
            _ => PasswordStrength::VeryWeak, // Default case
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_very_weak_password() {
        // Too short, no complexity
        assert_eq!(validate_password_strength(""), PasswordStrength::VeryWeak);
        assert_eq!(
            validate_password_strength("abc"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("123"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("pass"),
            PasswordStrength::VeryWeak
        );
    }

    #[test]
    fn test_weak_password() {
        // 8+ characters but low complexity (note: "password" is in insecure list)
        assert_eq!(
            validate_password_strength("abcdefgh"),
            PasswordStrength::Weak
        );
        assert_eq!(
            validate_password_strength("zzzzzzzz"),
            PasswordStrength::Weak
        );
        assert_eq!(
            validate_password_strength("aaaaaaaa"),
            PasswordStrength::Weak
        );
    }

    #[test]
    fn test_moderate_password() {
        // 8+ characters with some complexity (2 character types)
        assert_eq!(
            validate_password_strength("Testword"),
            PasswordStrength::Moderate
        );
        assert_eq!(
            validate_password_strength("test5678"),
            PasswordStrength::Moderate
        );
        assert_eq!(
            validate_password_strength("WORD9876"),
            PasswordStrength::Moderate
        );
    }

    #[test]
    fn test_strong_password() {
        // 12+ characters with good complexity (3+ character types)
        // 12 chars = 2 points, 3+ types = 2 points, total = 4 (VeryStrong)
        assert_eq!(
            validate_password_strength("Testword1234"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("MyTest123456"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("Secure@Word1"),
            PasswordStrength::VeryStrong
        );
    }

    #[test]
    fn test_very_strong_password() {
        // 16+ characters with excellent complexity
        assert_eq!(
            validate_password_strength("MySecureP@ssw0rd!"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("C0mpl3x!P@ssw0rd123"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("Str0ng&S3cur3P@ss"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("VeryL0ng!Secur3Passw0rd"),
            PasswordStrength::VeryStrong
        );
    }

    #[test]
    fn test_length_boundaries() {
        // Test length boundaries with all 4 complexity types
        // 6 chars: 0 length + 2 complexity = 2 (Moderate)
        assert_eq!(
            validate_password_strength("Test1!"),
            PasswordStrength::Moderate
        );
        // 8 chars: 1 length + 2 complexity = 3 (Strong)
        assert_eq!(
            validate_password_strength("Test1!ab"),
            PasswordStrength::Strong
        );
        // 12 chars: 2 length + 2 complexity = 4 (VeryStrong)
        assert_eq!(
            validate_password_strength("Test1!abcdef"),
            PasswordStrength::VeryStrong
        );
        // 16 chars: 3 length + 2 complexity = 5->4 (VeryStrong capped)
        assert_eq!(
            validate_password_strength("Test1!abcdefghij"),
            PasswordStrength::VeryStrong
        );
    }

    #[test]
    fn test_complexity_combinations() {
        // Only lowercase and uppercase (2 types): 8 chars + 2 types = score 2 (Moderate)
        assert_eq!(
            validate_password_strength("aBcDeFgH"),
            PasswordStrength::Moderate
        );

        // Lowercase, uppercase, and digits (3 types): 8 chars + 3 types = score 3 (Strong)
        assert_eq!(
            validate_password_strength("aBcD1234"),
            PasswordStrength::Strong
        );

        // All 4 types with shorter length: 0 length points + 2 complexity points = score 2 (Moderate)
        assert_eq!(
            validate_password_strength("aB1!"),
            PasswordStrength::Moderate
        );

        // All 4 types with 12+ length: 2 length points + 2 complexity points = score 4 (VeryStrong)
        assert_eq!(
            validate_password_strength("aB1!aB1!aB1!"),
            PasswordStrength::VeryStrong
        );
    }

    #[test]
    fn test_special_characters() {
        // Various special characters (14 chars, 4 types = score 4)
        assert_eq!(
            validate_password_strength("Test@123456789"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("Test#123456789"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("Test$123456789"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("Test!123456789"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("Test_123456789"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("Test-123456789"),
            PasswordStrength::VeryStrong
        );
    }

    #[test]
    fn test_unicode_characters() {
        // Unicode characters should be treated as special characters (12+ chars, 4 types = score 4)
        assert_eq!(
            validate_password_strength("Tëstw0rd1234"),
            PasswordStrength::VeryStrong
        );
        assert_eq!(
            validate_password_strength("密码Testword123"),
            PasswordStrength::VeryStrong
        );
    }

    #[test]
    fn test_common_insecure_passwords() {
        // Test that common passwords are always rated as very weak
        assert_eq!(
            validate_password_strength("password"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("Password"),
            PasswordStrength::VeryWeak
        ); // Case insensitive
        assert_eq!(
            validate_password_strength("PASSWORD"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("123456"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("qwerty"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("letmein"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("iloveyou"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("dragon"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("monkey"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("sunshine"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("12345678"),
            PasswordStrength::VeryWeak
        );
    }

    #[test]
    fn test_modified_common_passwords() {
        // Modified versions of common passwords should score higher
        assert_eq!(
            validate_password_strength("P@ssw0rd!"),
            PasswordStrength::Strong
        ); // 9 chars, 4 types = score 3
        assert_eq!(
            validate_password_strength("Passw0rd123456!"),
            PasswordStrength::VeryStrong
        ); // 15 chars, 4 types = score 4
        assert_eq!(
            validate_password_strength("987654!Abc"),
            PasswordStrength::Strong
        ); // 10 chars, 4 types = score 3
    }

    #[test]
    fn test_insecure_list_coverage() {
        // Ensure multiple entries from the insecure list are caught
        let insecure_passwords = vec![
            "12345678",
            "qwertyuiop",
            "abc123",
            "football",
            "baseball",
            "superman",
            "batman",
            "trustno1",
            "killer",
            "master",
        ];

        for pwd in insecure_passwords {
            assert_eq!(
                validate_password_strength(pwd),
                PasswordStrength::VeryWeak,
                "Password '{}' should be rated as very weak",
                pwd
            );
        }
    }

    #[test]
    fn test_password_strength_enum() {
        // Test the enum values match expected integers
        assert_eq!(PasswordStrength::VeryWeak as u8, 0);
        assert_eq!(PasswordStrength::Weak as u8, 1);
        assert_eq!(PasswordStrength::Moderate as u8, 2);
        assert_eq!(PasswordStrength::Strong as u8, 3);
        assert_eq!(PasswordStrength::VeryStrong as u8, 4);
    }

    #[test]
    fn test_from_u8_conversion() {
        // Test the From<u8> implementation
        assert_eq!(PasswordStrength::from(0), PasswordStrength::VeryWeak);
        assert_eq!(PasswordStrength::from(1), PasswordStrength::Weak);
        assert_eq!(PasswordStrength::from(2), PasswordStrength::Moderate);
        assert_eq!(PasswordStrength::from(3), PasswordStrength::Strong);
        assert_eq!(PasswordStrength::from(4), PasswordStrength::VeryStrong);
        assert_eq!(PasswordStrength::from(5), PasswordStrength::VeryWeak); // Out of range defaults to VeryWeak
        assert_eq!(PasswordStrength::from(255), PasswordStrength::VeryWeak);
    }
}
