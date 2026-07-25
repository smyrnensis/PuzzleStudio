# Agent Notes

This crate owns the small platform-neutral audio wire contract: asset and voice
identities, resolved lifecycle commands, capability states, and device
commands.

It must not depend on synthesis, authoring recipes, session state, Bevy,
WebAudio, DOM, clocks, files, or JSON transport.
