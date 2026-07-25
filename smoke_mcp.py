#!/usr/bin/env python3
"""Smoke-test a stdio MCP server: call its `health` tool + one extra tool.

Usage: python smoke_mcp.py <name> <command> [env=val ...] [extra_tool=json_args]
The extra tool is the last positional arg, parsed as JSON (or "{}").
"""
import asyncio
import json
import os
import sys

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


async def main():
    name = sys.argv[1]
    command = sys.argv[2]
    rest = sys.argv[3:]
    env_overrides = {}
    extra = None
    for a in rest:
        if "=" in a and not a.startswith("{"):
            k, v = a.split("=", 1)
            env_overrides[k] = v
        else:
            extra = a  # JSON args for an extra tool call

    env = dict(os.environ)
    env.update({k: v.replace("__HOME__", os.path.expanduser("~")) for k, v in env_overrides.items()})

    params = StdioServerParameters(command=command, args=[], env=env)
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            names = [t.name for t in tools.tools]
            print(f"[{name}] tools({len(names)}): {', '.join(names)}")
            # health
            if "health" in names:
                r = await session.call_tool("health", {})
                print(f"[{name}] health -> {r.content[0].text if r.content else r}")
            # extra tool
            if extra:
                tool_name, _, args_json = extra.partition(":")
                args = json.loads(args_json) if args_json else {}
                if tool_name in names:
                    r = await session.call_tool(tool_name, args)
                    txt = r.content[0].text if r.content else str(r)
                    print(f"[{name}] {tool_name} -> {txt[:300]}")


asyncio.run(main())