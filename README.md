# aidash — AI Coding Assistant Token & Cost Dashboard

> A rich terminal dashboard for tracking token usage, costs, and session metrics across GitHub Copilot CLI and Claude Code.

Built in Rust with [ratatui](https://ratatui.rs/) — line charts, bar charts, live monitoring, and multi-view navigation.

## Features

### Dashboard
- **Sessions table** — All sessions at a glance with status, name, model, tokens, cost, turns, agents, duration
- **Multi-column sorting** — Press `s` to cycle, `S` to reverse, or `1`–`9` to jump to a specific column
- **Source filtering** — Toggle between All / Copilot / Claude with `Tab`
- **Active sessions first** — Default sort groups active (● ON) sessions at the top

### Session Detail (`Enter`)
- **Token breakdown** — Output, estimated input, reasoning tokens, total, and cost
- **Output vs Input ratio bar** — Visual split of token distribution
- **Per-turn token chart** — Line chart (`Chart` widget with Braille markers) showing token usage over turns
- **Models used** — List of models with message counts, colored by tier
- **Sub-agents summary** — Grouped by type with total duration
- **Top tools** — Mini bar chart of the 10 most-used tools
- **Code changes** — Lines added/removed, files modified

### Live Dashboard (`L` / `l`)
- **4-quadrant real-time view** for active sessions, auto-refreshing every 2 seconds
- **Token accumulation** — Dual-line chart: per-turn tokens + cumulative growth curve
- **Tool activity** — Horizontal bar chart of tool call frequency
- **Model distribution** — Bar chart of messages per model
- **Sub-agents panel** — Grouped stats + currently running agents with elapsed time
- **Rate metrics** — Tokens/min, live cost ticker, turn count

### Additional Views
- **Agents** (`a`) — Detailed table of all sub-agent tasks with type, timing, duration, status
- **Tools** (`t`) — Full bar chart with gradient colors, percentages, category-colored names
- **Models** (`m`) — Per-model message distribution, cost attribution, and details table
- **Trends** (`T`) — 30-day line chart (tokens + cost overlay), model distribution, top tools, session timeline
- **Help** (`?`) — Full keybinding reference overlay

### Themes
- **Dark theme** (default) — Warm gold/amber/lime palette on dark background
- **Light theme** (`--light` flag or `d` key) — Deep tones on warm white background
- Toggle at runtime with `d` key

### CLI Commands
```
aidash                    # Launch interactive TUI
aidash list               # Print sessions table
aidash list --json        # JSON output for scripting
aidash session <id>       # Session details (non-interactive)
aidash cost               # Total cost summary
aidash cost --since DATE  # Cost since YYYY-MM-DD
aidash init               # Create ~/.aidash/pricing.json
aidash update-pricing     # Refresh pricing with latest defaults
aidash --source copilot   # Filter to Copilot sessions only
aidash --source claude    # Filter to Claude sessions only
aidash --light            # Start with light theme
```

## Installation

### From source
```bash
git clone https://github.com/user/aidash.git
cd aidash
cargo build --release
# Binary at target/release/aidash
```

### Requirements
- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- No other dependencies — SQLite is bundled

## Keyboard Shortcuts

### Dashboard
| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate sessions |
| `Enter` | Drill into selected session |
| `s` | Cycle sort column |
| `S` | Reverse sort direction |
| `1`–`9` | Sort by column N directly (toggles direction) |
| `Tab` | Switch source: All → Copilot → Claude |
| `L` | Live dashboard (active sessions only) |
| `T` | Trends & analytics |
| `r` | Refresh data from disk |
| `d` | Toggle dark/light theme |
| `?` | Help overlay |
| `q` | Quit |

### Session Detail
| Key | Action |
|-----|--------|
| `a` | Sub-agents breakdown |
| `t` | Tool usage breakdown |
| `m` | Models breakdown |
| `l` | Live dashboard (active sessions) |
| `Esc` | Back to dashboard |

### All Views
| Key | Action |
|-----|--------|
| `d` | Toggle theme |
| `Esc` / `Backspace` | Go back |
| `q` | Quit |

## Data Sources

### GitHub Copilot CLI (`~/.copilot/`)
| File | Data |
|------|------|
| `session-state/<id>/events.jsonl` | Token counts, sub-agents, tool calls, model changes |
| `session-state/<id>/workspace.yaml` | Session metadata (summary, branch, cwd) |
| `session-store.db` | SQLite — session summaries, turns |
| `config.json` | Current model selection |

### Claude Code (`~/.claude/`)
| File | Data |
|------|------|
| `projects/<project>/<session>.jsonl` | Conversation history, token usage, cost |
| `history.jsonl` | Global session index |

## Custom Pricing

Pricing is stored in `~/.aidash/pricing.json` (auto-created on first run).

Edit it to add custom models or override prices:

```json
{
  "claude-opus-4.6-1m": {
    "input_per_million": 15.0,
    "output_per_million": 75.0,
    "is_premium": true
  },
  "my-custom-model": {
    "input_per_million": 1.0,
    "output_per_million": 5.0,
    "is_premium": false
  }
}
```

Run `aidash update-pricing` to merge the latest default prices into your config without losing custom entries.

## Architecture

```
src/
├── main.rs              # CLI entry point (clap)
├── app.rs               # TUI state machine, event loop, view routing
├── config.rs            # Pricing configuration (~/.aidash/pricing.json)
├── util.rs              # Number/time/string formatting helpers
├── data/
│   ├── models.rs        # Unified Session, Turn, SubAgent, ToolCall types
│   ├── copilot.rs       # Copilot CLI event parser (events.jsonl + SQLite)
│   └── claude.rs        # Claude Code session parser (JSONL)
├── cost/
│   └── estimator.rs     # Token estimation & cost calculation
└── tui/
    ├── theme.rs         # Dark/light theme system with warm color palette
    ├── dashboard.rs     # Sessions overview table with sorting
    ├── session.rs       # Session detail (charts, token breakdown)
    ├── live.rs          # Real-time monitoring dashboard (4 quadrants)
    ├── agents.rs        # Sub-agents detail table
    ├── tools.rs         # Tool usage bar chart
    ├── models.rs        # Per-model analysis with cost attribution
    ├── trends.rs        # 30-day trends, sparklines, timeline
    └── help.rs          # Help overlay
```

## Token Estimation

Since exact input tokens aren't tracked in Copilot events, aidash estimates:
- **Output tokens** — Directly from `assistant.message.outputTokens`
- **Input tokens** — Estimated from user message content length (~4 chars/token)
- **Reasoning tokens** — Estimated from `reasoningText` length
- **Cost** — Calculated per-model using the pricing table, distributed proportionally by message count

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo build` and `cargo clippy` to verify
5. Submit a pull request

## License

MIT
