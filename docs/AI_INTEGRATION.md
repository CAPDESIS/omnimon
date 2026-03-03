# AI Integration

macmon v2 keeps execution native and safe.

## Data Flow

1. User opens Preferences and selects provider and model.
2. API key is saved with `SecItemAdd` in macOS Keychain.
3. User presses Smart Optimize in Process Picker.
4. Swift builds a lightweight top process snapshot.
5. Snapshot is sent to selected AI provider for analysis only.
6. Provider returns strict JSON with candidate PIDs.
7. UI marks suggested rows and asks user to apply or review.
8. Only after approval, selected PIDs are passed to Bash.
9. Bash validates process safety and sends `kill -15` with `kill -9` fallback.

## Security Rules

1. AI output is treated as untrusted input.
2. Only PIDs are accepted from AI response.
3. No command string from AI is executed.
4. Blocklist and Apple system process protections are always enforced.
5. Protected audio and video services stay untouchable.

## Providers

1. OpenAI
2. Anthropic
3. OpenRouter

## Keychain Storage

1. Service key: `com.macmon.ai`
2. Account key: provider name (`openai`, `anthropic`, `openrouter`)
3. Value: API key bytes, never plain text file storage
