"""Never-before-seen synthetic config flow actually executed by official Core."""
from homeassistant import config_entries
from homeassistant.data_entry_flow import FlowResult
import voluptuous as vol

DOMAIN = "hars_lifecycle_probe"

class HarsLifecycleConfigFlow(config_entries.ConfigFlow, domain=DOMAIN):
    VERSION = 1

    async def async_step_user(self, user_input=None) -> FlowResult:
        if user_input is None:
            return self.async_show_form(
                step_id="user",
                data_schema=vol.Schema({vol.Required("label"): str}),
            )
        await self.async_set_unique_id("synthetic_lifecycle_only")
        self._abort_if_unique_id_configured()
        return self.async_create_entry(
            title="Fictional Core lifecycle",
            data={"label": user_input["label"]},
        )
