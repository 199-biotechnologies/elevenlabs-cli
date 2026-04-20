# Fixer γ — phone endpoints SDK contract fixes

Ground truth: elevenlabs-python raw_client.py for
[whatsapp](https://github.com/elevenlabs/elevenlabs-python/blob/main/src/elevenlabs/conversational_ai/whatsapp/raw_client.py)
and [batch_calls](https://github.com/elevenlabs/elevenlabs-python/blob/main/src/elevenlabs/conversational_ai/batch_calls/raw_client.py).

## Checklist

- [x] `phone whatsapp call` body → SDK shape (`whatsapp_phone_number_id`,
      `whatsapp_user_id`, `whatsapp_call_permission_request_template_{name,language_code}`,
      `agent_id`).
- [x] `phone whatsapp message` body → SDK shape (same ids,
      `template_name`, `template_language_code`, `template_params`
      as a single body-component with named text params, `agent_id`,
      optional `conversation_initiation_client_data`). `--text` dropped.
- [x] `phone batch submit`: body field `name` → `call_name` (internal;
      `--name` flag unchanged).
- [x] `phone batch list`: query `page_size`→`limit`, `cursor`→`last_doc`;
      accepts `_status`/`_agent_id` but drops them (SDK has no filter).
- [x] Tests rewritten in all three test files to pin the new shapes.
- [x] cli.rs snippet for `PhoneWhatsappAction` in sibling `cli.rs`.

## Lead must splice (cargo will NOT build until done)

1. **src/cli.rs** — replace `PhoneWhatsappAction` variants per sibling
   snippet. Drop `PhoneBatchAction::List::{status, agent_id}` + their
   `#[arg(long)]` flags.
2. **src/commands/phone/mod.rs** — update `dispatch_whatsapp` to
   destructure new fields and call `call::run`/`message::run` with the
   new params. Update `PhoneBatchAction::List` branch to drop
   `status`/`agent_id` (or pass `None, None`).

## CHANGELOG (breaking, v0.2.0)

```
phone whatsapp call:    --whatsapp-account/--recipient → --whatsapp-phone-number/--whatsapp-user
                        + required --permission-template/-language
phone whatsapp message: same id rename; --text REMOVED (approved templates only);
                        + required --template/-language, repeatable --template-param key=value,
                        optional --client-data <json_file>
phone batch submit:     wire `name` → `call_name`. --name flag unchanged.
phone batch list:       --status/--agent-id REMOVED (no SDK filter).
                        wire `page_size`→`limit`, `cursor`→`last_doc`.
```
