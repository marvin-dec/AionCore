-- Add StarCLI as a builtin ACP agent.
--
-- StarCLI is a global-install ACP agent: `npm install -g starcli` places `star`
-- on PATH, and `star acp` is the ACP server entrypoint. `star --version` is
-- documented, so the default PATH probe applies.
--
-- `binary_name` is `star`: the official CLI binary name after global install.
-- No `bridge_binary` needed — star is launched directly, not via npx.
--
-- agent_capabilities uses the conservative baseline (load_session true, MCP
-- stdio/http/sse) until a live probe confirms the exact handshake shape; the
-- management view will refresh capabilities on first connection.
--
-- yolo_id stays NULL: no known full-auto mode id until probe confirms.
-- native_skills_dirs stays NULL: no known project-relative skills directory.
-- behavior_policy omits supports_team: migration 033 retired the negative
-- form, and team capability is derived from backend + probed capabilities.
-- Post-030 seed shape: builtin rows use agent_id = id and user_id NULL.
INSERT INTO agent_metadata
    (id, agent_id, icon, name, backend, agent_type, agent_source, agent_source_info,
     enabled, command, args, env, native_skills_dirs, behavior_policy, yolo_id,
     agent_capabilities, sort_order, created_at, updated_at)
VALUES
    ('b1a7c5f2', 'b1a7c5f2', '/api/assets/logos/acp-registry/starcli.svg', 'StarCLI',
     'starcli', 'acp', 'builtin', '{"binary_name":"star"}',
     1, 'star', '["acp"]', '[]',
     NULL,
     '{"supports_side_question":false}',
     NULL,
     '{"load_session":true,"mcp_capabilities":{"stdio":true,"http":true,"sse":true},"prompt_capabilities":{"image":false,"audio":false,"embedded_context":false},"session_capabilities":{"list":{},"fork":{},"resume":{},"close":{}}}',
     3360,
     unixepoch('now','subsec')*1000, unixepoch('now','subsec')*1000)
ON CONFLICT(id) DO UPDATE SET
    agent_id = excluded.agent_id,
    icon = excluded.icon,
    name = excluded.name,
    description = NULL,
    backend = excluded.backend,
    agent_type = excluded.agent_type,
    agent_source = excluded.agent_source,
    agent_source_info = excluded.agent_source_info,
    enabled = excluded.enabled,
    command = excluded.command,
    args = excluded.args,
    env = excluded.env,
    native_skills_dirs = excluded.native_skills_dirs,
    behavior_policy = excluded.behavior_policy,
    yolo_id = excluded.yolo_id,
    sort_order = excluded.sort_order,
    updated_at = unixepoch('now','subsec')*1000;
