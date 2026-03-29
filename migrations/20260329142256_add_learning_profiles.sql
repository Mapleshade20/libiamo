CREATE TABLE user_learning_profiles (
    user_id           INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    language          language_code NOT NULL,

    level_self_assign INT NOT NULL DEFAULT 2 CHECK (level_self_assign BETWEEN 1 AND 5),

    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (user_id, language)
);

CREATE INDEX idx_user_learning_profiles_user
  ON user_learning_profiles(user_id);
