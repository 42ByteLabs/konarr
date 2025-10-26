//! # User Registration
use crate::{
    KonarrError,
    models::{ServerSettings, SessionState, SessionType, Sessions, Setting, UserRole, Users},
    utils::password::{PasswordStrength, validate_password_strength},
};
use geekorm::{ConnectionManager, prelude::*};

/// User Registration Request
#[derive(Debug)]
pub struct UserRegistrationRequest {
    /// Username
    ///
    /// **Requirements:**
    ///
    /// - must be unique
    /// - not empty, and between 3 and 30 characters
    /// - can only contain alphanumeric characters, underscores, dots, and hyphens
    ///
    ///
    pub username: String,
    /// Password
    ///
    /// **Requirements:**
    /// - at least 12 characters long
    /// - contains at least one uppercase letter, one lowercase, one digit, and one special character
    pub password: String,
    /// User Role, (default: User)
    pub role: Option<UserRole>,
}

impl Users {
    /// Check if Registration is Open or Closed
    pub async fn open_registration(database: &ConnectionManager) -> Result<bool, KonarrError> {
        let registration: String =
            ServerSettings::fetch_by_name(&database.acquire().await, Setting::Registration)
                .await?
                .value;

        Ok(registration.to_lowercase() == "enabled")
    }

    /// User Registration
    ///
    /// Creates a new user based on the registration request provided.
    ///
    ///
    pub async fn register(
        database: &ConnectionManager,
        request: UserRegistrationRequest,
    ) -> Result<Users, KonarrError> {
        // Validate username uniqueness
        // TODO:
        if Users::fetch_by_username(&database.acquire().await, &request.username)
            .await
            .is_ok()
        {
            return Err(KonarrError::RegistrationError(format!(
                "Username '{}' is already taken.",
                request.username
            )));
        }

        if !validate_username(&request.username.as_str()) {
            return Err(KonarrError::RegistrationError(format!(
                "Username '{}' is invalid. It must be between 3 and 30 characters and can only contain alphanumeric characters, underscores, dots, and hyphens.",
                request.username
            )));
        }

        // Validate password, the admin may have set a minimum strength requirement
        let password_strength_requirement: PasswordStrength = match ServerSettings::fetch_by_name(
            &database.acquire().await,
            Setting::PasswordStrength,
        )
        .await
        {
            Ok(setting) => setting.value.parse::<u8>().unwrap_or(3).into(),
            Err(_) => PasswordStrength::Strong,
        };

        let password_strength = validate_password_strength(request.password.as_str());

        if password_strength < password_strength_requirement {
            return Err(KonarrError::RegistrationError(format!(
                "Password does not meet the required strength of {:?}.",
                password_strength_requirement
            )));
        }

        // Everything is valid, create the user

        // Create Session
        let mut session = Sessions::new(SessionType::User, SessionState::Active);
        session.save(&database.acquire().await).await?;

        let mut user = Users::new(
            request.username.clone(),
            request.password.clone(),
            request.role.unwrap_or_default(),
            session.id,
        );
        user.save(&database.acquire().await).await?;

        Ok(user)
    }
}

/// Validate Username
/// - between 3 and 30 characters
/// - can only contain alphanumeric characters, underscores, dots, and hyphens
fn validate_username(username: &str) -> bool {
    let len = username.len();
    if len < 3 || len > 30 {
        return false;
    }
    username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_username_accepts_basic_alphanumeric_and_symbols() {
        assert!(validate_username("user123"));
        assert!(validate_username("john_doe"));
        assert!(validate_username("john.doe-1990"));
        assert!(validate_username("_.-")); // underscores, dots, hyphens allowed
    }

    #[test]
    fn validate_username_rejects_too_short_or_empty() {
        assert!(!validate_username(""));
        assert!(!validate_username("ab")); // < 3
    }

    #[test]
    fn validate_username_accepts_min_and_max_lengths() {
        let min = "abc"; // 3 chars
        assert!(validate_username(min));

        let max = "a".repeat(30);
        assert!(validate_username(&max));

        let over = "a".repeat(31);
        assert!(!validate_username(&over));
    }

    #[test]
    fn validate_username_rejects_disallowed_characters() {
        assert!(!validate_username("bad user")); // spaces not allowed
        assert!(!validate_username("user!")); // punctuation not in allowed set
        assert!(!validate_username("name@domain")); // @ not allowed
        assert!(!validate_username("user🙂")); // emoji not allowed
    }

    #[test]
    fn validate_username_unicode_letters_are_allowed_by_current_impl() {
        // Note: current implementation uses char::is_alphanumeric which accepts many Unicode letters.
        // This test documents that behavior.
        assert!(validate_username("usér")); // 'é' is considered alphanumeric
        assert!(validate_username("用户123")); // CJK letters are considered alphanumeric by is_alphanumeric
    }
}
