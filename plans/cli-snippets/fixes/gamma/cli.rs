//! src/cli.rs — `PhoneWhatsappAction` replacement for v0.2.0
//!
//! The lead must splice this enum in place of the existing
//! `PhoneWhatsappAction` definition (around lines 1423-1468 of
//! src/cli.rs in the pre-fix tree) AND update `src/commands/phone/mod.rs`
//! (`dispatch_whatsapp`) so the new fields destructure correctly.
//!
//! Flag renames vs. the pre-fix CLI (MUST appear in CHANGELOG):
//!
//!   phone whatsapp call
//!     - `--whatsapp-account <id>`  → `--whatsapp-phone-number <id>`  (field rename)
//!     - `--recipient <phone>`       → `--whatsapp-user <id>`          (field rename)
//!     - NEW required: `--permission-template <name>`
//!     - NEW required: `--permission-template-language <code>`
//!
//!   phone whatsapp message
//!     - `--whatsapp-account <id>`  → `--whatsapp-phone-number <id>`  (field rename)
//!     - `--recipient <phone>`       → `--whatsapp-user <id>`          (field rename)
//!     - DROPPED: `--text`            (WhatsApp only accepts approved templates)
//!     - NEW required: `--template <name>`
//!     - NEW required: `--template-language <code>`        (e.g. `en_US`)
//!     - NEW repeatable: `--template-param <key>=<value>`   (body component text params)
//!     - NEW optional: `--client-data <json_file>`          (conversation_initiation_client_data)
//!
//! Nothing changes for `phone batch submit|list|…` — those use internal
//! field-name remapping only (see the respective command module fixes).

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneWhatsappAction {
    /// Place an outbound WhatsApp voice call.
    ///
    /// WhatsApp requires a pre-approved "call permission request" template
    /// — the recipient must have previously granted call consent. Supply
    /// both the template name and its language code (e.g. `en_US`).
    Call {
        /// Agent ID to handle the call
        #[arg(long = "agent")]
        agent_id: String,

        /// WhatsApp phone number ID to call from
        /// (the sending business number, not the recipient)
        #[arg(long = "whatsapp-phone-number")]
        whatsapp_phone_number_id: String,

        /// WhatsApp user ID of the recipient
        #[arg(long = "whatsapp-user")]
        whatsapp_user_id: String,

        /// Name of the pre-approved WhatsApp call-permission-request template
        #[arg(long = "permission-template")]
        permission_template_name: String,

        /// Language code for the permission template (e.g. `en_US`, `es`)
        #[arg(long = "permission-template-language")]
        permission_template_language_code: String,
    },

    /// Send an outbound WhatsApp message via a pre-approved template.
    ///
    /// WhatsApp's platform rules require every outbound message to use a
    /// pre-approved template — free-form text is rejected. Supply the
    /// template name, language code, and any `{{name}}`-substituted body
    /// parameters via repeated `--template-param key=value`.
    Message {
        /// Agent ID associated with the message
        #[arg(long = "agent")]
        agent_id: String,

        /// WhatsApp phone number ID to send from
        /// (the sending business number, not the recipient)
        #[arg(long = "whatsapp-phone-number")]
        whatsapp_phone_number_id: String,

        /// WhatsApp user ID of the recipient
        #[arg(long = "whatsapp-user")]
        whatsapp_user_id: String,

        /// Name of the pre-approved WhatsApp template
        #[arg(long = "template")]
        template_name: String,

        /// Language code for the template (e.g. `en_US`, `es`)
        #[arg(long = "template-language")]
        template_language_code: String,

        /// Template body parameter, as `key=value`. Repeat once per
        /// placeholder. All params are sent as a single `body` component
        /// with named text parameters — matches the WhatsApp Cloud API
        /// body-parameter convention.
        #[arg(long = "template-param", value_name = "KEY=VALUE")]
        template_params: Vec<String>,

        /// Optional path to a JSON file whose contents become the
        /// `conversation_initiation_client_data` field (dynamic_variables,
        /// overrides, etc.). Passed through verbatim.
        #[arg(long = "client-data")]
        client_data: Option<String>,
    },

    /// Manage WhatsApp accounts
    Accounts {
        #[command(subcommand)]
        action: PhoneWhatsappAccountsAction,
    },
}
