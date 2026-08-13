# Bust-Packer

Usually you don't want your project to be a monolithic text file — but sometimes it's convenient.

**Bust-Packer** turns an entire project directory into a single, clean `.txt` file (non-destructively). This makes it easy to:

- Share a project without sending a full repository
- Send a project to an AI agent for review or continued development
- Keep a lightweight, portable version of your work

You can also unpack the `.txt` file back into a usable project directory.

### Features

- Convert any project directory into one monolithic text file
- Non-destructive (original files remain untouched)
- Optional inclusion of a specialized AI prompt designed for building projects
- Unpack the text file back into a full directory structure
- Lightweight and simple

The included AI prompt works particularly well in Linux Ubuntu terminals (other operating systems have not been fully tested).

---

### Requirements

- [Rust](https://www.rust-lang.org/tools/install) (latest stable recommended)

---

### Build & Run

```bash
# Clone the repository
git clone https://github.com/byteslip-applications/Bust-Packer.git
cd Bust-Packer

# Run the application
cargo run --release

```
### License

This project is licensed under the **Business Source License 1.1**.

- **Change Date:** 2029-01-01  
- **Change License:** Apache License 2.0

Until the Change Date, use of this software is governed by the Business Source License 1.1. After that date, the project will be available under the Apache-2.0 license.

The full license text is available in the `LICENSE` file and is also included in the application (`src/app/license.rs`).


###While using the AI prompted text monolithic text file:
- Use caution when pasting code into your terminal.
- The AI will be prompted to provide code that edits your project files via terminal commands.
- The terminal commands will also create a temporary log and copy the log automatically to your clipboard.
- The terminal commands will automatically delete the temporary log file after copying.
- Once you execute a command from the AI in your terminal, simply return to the AI and ctrl + v to paste into the chat to report the results back to the AI.


##This is the most convenient way I have found for using AI assistants for project progression, even more so than actual coding terminal agents, but this is just my opinion.
##What works for you will work best for you, and everyone has different experiences. I just found this one is very effective for the way I progress projects, and I have tried alot of ways.
