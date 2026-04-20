// ── Phone ──────────────────────────────────────────────────────────────────
//
// REPLACE the existing PhoneAction enum with this, and ADD the three sub-enums
// below (PhoneBatchAction, PhoneWhatsappAction, PhoneWhatsappAccountsAction).
// The existing `List` and `Call` variants are preserved verbatim.

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneAction {
    /// List phone numbers
    #[command(visible_alias = "ls")]
    List,

    /// Make an outbound call with an agent
    Call {
        /// Agent ID to handle the call
        agent_id: String,

        /// Phone number ID to call from
        #[arg(long)]
        from_id: String,

        /// E.164 number to call (+1...)
        #[arg(long)]
        to: String,
    },

    /// Batch outbound calls (CSV or JSON recipients)
    Batch {
        #[command(subcommand)]
        action: PhoneBatchAction,
    },

    /// WhatsApp channel: outbound calls, messages, and accounts
    Whatsapp {
        #[command(subcommand)]
        action: PhoneWhatsappAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneBatchAction {
    /// Submit a batch of outbound calls.
    ///
    /// `--recipients` accepts a path to a CSV or JSON file (or `-` for stdin).
    /// CSV format: `phone_number,conversation_initiation_client_data` with an
    /// optional header row. The data column holds a JSON blob (or is empty).
    /// JSON format: an array of `{phone_number, conversation_initiation_client_data?}` objects.
    Submit {
        /// Agent ID that will handle the calls
        #[arg(long = "agent")]
        agent_id: String,

        /// Phone number ID to dial from
        #[arg(long = "phone-number")]
        phone_number_id: String,

        /// Path to CSV or JSON recipients file (use `-` for stdin)
        #[arg(long)]
        recipients: String,

        /// Optional human-readable batch name
        #[arg(long)]
        name: Option<String>,

        /// Optional scheduled start time as a Unix timestamp
        #[arg(long, value_name = "UNIX")]
        scheduled_time_unix: Option<i64>,
    },

    /// List batch calls in the current workspace
    #[command(visible_alias = "ls")]
    List {
        /// Page size (1-100)
        #[arg(long)]
        page_size: Option<u32>,

        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,

        /// Filter by batch status
        #[arg(long)]
        status: Option<String>,

        /// Filter by agent ID
        #[arg(long)]
        agent_id: Option<String>,
    },

    /// Show detail for a batch (includes per-call status)
    #[command(visible_alias = "get")]
    Show {
        /// Batch ID
        batch_id: String,
    },

    /// Cancel a batch (reversible via `phone batch retry`)
    Cancel {
        /// Batch ID
        batch_id: String,
    },

    /// Retry a batch (re-dials failed/pending recipients)
    Retry {
        /// Batch ID
        batch_id: String,
    },

    /// Delete a batch
    #[command(visible_alias = "rm")]
    Delete {
        /// Batch ID
        batch_id: String,

        /// Confirm deletion. Required because it is irreversible.
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneWhatsappAction {
    /// Place an outbound WhatsApp voice call
    Call {
        /// Agent ID to handle the call
        #[arg(long = "agent")]
        agent_id: String,

        /// WhatsApp account ID to call from
        #[arg(long = "whatsapp-account")]
        whatsapp_account: String,

        /// Recipient phone number in E.164 format (+1...)
        #[arg(long)]
        recipient: String,
    },

    /// Send an outbound WhatsApp message (free-form text OR a pre-approved template)
    Message {
        /// Agent ID associated with the message
        #[arg(long = "agent")]
        agent_id: String,

        /// WhatsApp account ID to send from
        #[arg(long = "whatsapp-account")]
        whatsapp_account: String,

        /// Recipient phone number in E.164 format (+1...)
        #[arg(long)]
        recipient: String,

        /// Free-form message text (mutually exclusive with --template)
        #[arg(long, conflicts_with = "template")]
        text: Option<String>,

        /// Pre-approved WhatsApp template name (mutually exclusive with --text)
        #[arg(long)]
        template: Option<String>,
    },

    /// Manage WhatsApp accounts
    Accounts {
        #[command(subcommand)]
        action: PhoneWhatsappAccountsAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneWhatsappAccountsAction {
    /// List WhatsApp accounts
    #[command(visible_alias = "ls")]
    List,

    /// Show details for a WhatsApp account
    #[command(visible_alias = "get")]
    Show {
        /// WhatsApp account ID
        account_id: String,
    },

    /// PATCH a WhatsApp account with partial JSON from a file
    Update {
        /// WhatsApp account ID
        account_id: String,

        /// Path to a JSON file whose contents become the PATCH body
        #[arg(long)]
        patch: String,
    },

    /// Delete a WhatsApp account
    #[command(visible_alias = "rm")]
    Delete {
        /// WhatsApp account ID
        account_id: String,

        /// Confirm deletion. Required because it is irreversible.
        #[arg(long)]
        yes: bool,
    },
}
