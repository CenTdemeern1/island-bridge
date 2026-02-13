# IslandBridge

Dead simple [Archipelago](archipelago.gg) to (Discord) webhook bridge.

Useful for tracking and discussing hints with your entire team.

## TeamTracker

This client is a "TeamTracker", which means it can use the functionality I implemented in this fork: [CenTdemeern1/Archipelago](https://github.com/CenTdemeern1/Archipelago)

This allows it to see hints for the entire team, instead of just the slot it's connected to, for easier coordination and discussion.

### Why?

Practical example of this:
> IslandBridge - 10:00 AM
> 
> [Hint]: **PlayerC**'s ***__ProgressionItem__*** is at **We Need To Go Deeper** in **PlayerB**'s World. (priority)

> PlayerA - 10:01 AM
> 
> Oh wait `@PlayerB` I found your **Flint And Steel Recipes** in a shop I'll get them for you so you can help `@PlayerC`

If IslandBridge is connected to PlayerA's slot, it would not be able to see the aforementioned hint if it wasn't a TeamTracker.

Hints in chat are normally only visible to the slot the item belongs to, and the slot the item is in, presumably to prevent unnecessary clutter for the reader.

# How to use it

Set the environment variables, and run it. (Install Rust via https://rustup.rs/, then `cargo run`)

## Environment variables

Consider putting these in a file and `source`ing it.

- `ISLANDBRIDGE_WEBHOOK`: The webhook URL.
- `ISLANDBRIDGE_AP_URL`: The Archipelago server URL.
- `ISLANDBRIDGE_AP_SLOT`: The Archipelago slot to connect to.
- `ISLANDBRIDGE_AP_PASSWORD`: If set, the Archipelago password.

### Example environment variable file

```zsh
export ISLANDBRIDGE_WEBHOOK="https://discord.com/api/webhooks/0123456789/abc"
export ISLANDBRIDGE_AP_URL="archipelago.gg:38281"
export ISLANDBRIDGE_AP_SLOT="PlayerA"
unset ISLANDBRIDGE_AP_PASSWORD # No password
```
