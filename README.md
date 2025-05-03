# mcp-mutinynet

An MCP server for utilizing the mutinynet faucet.

## Setup

1. Install

```
git clone https://github.com/benthecarman/mcp-mutinynet.git
cd mcp-mutinynet
cargo install --path .
```

2. Add to mcp config

```
{
    "mcpServers": {
       "mcp-mutinynet": {
          "command": "mcp-mutinynet",
          "args": [
             "--mcp"
          ]
       }
    }
 }
```
