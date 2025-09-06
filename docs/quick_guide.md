## 1. Create Knowledge-Base

First, let's say there're text files explaining ai. We'll build a knowledge-base with from the text files. The directory should look like below.

```
ai_tutorials/
  |
  *-- ai_tutorial_1.txt
  |
  *-- ai_tutorial_2.txt
  |
  *-- ai_tutorial_3.txt
  |
  *-- ... and many more txt files
```

Run `cd ai_tutorial; rag init`. You'll see a new directory created like below.

```
ai_tutorials/
  |
  *-- .ragit/
  |   |
  |   *-- chunks/
  |   |
  |   *-- configs/
  |   |
  |   *-- files/
  |   |
  |   *-- images/
  |   |
  |   *-- prompts/
  |   |
  |   *-- index.json
  |   |
  |   *-- models.json
  |
  *-- ai_tutorial_1.txt
  |
  *-- ai_tutorial_2.txt
  |
  *-- ai_tutorial_3.txt
  |
  *-- ... and many more txt files
```

`.ragit/` is like `.git/` of git repositories. It saves metadata and chunks. After `rag init`, the knowledge-base is empty. You have to add files to the staging using `rag add` command.

Run `rag add --all`. Now you're ready to build the knowledge-base. Run `rag build` to start the work. The default model is `llama3.3-70b-groq` and you need `GROQ_API_KEY` to run. If you want to run gpt-4o-mini, run `rag config --set model gpt-4o-mini`. You can see the list of the models using `rag ls-models`. You can also add models manually to `.ragit/models.json`.

```
elapsed time: 00:33
staged files: 15, processed files: 13
errors: 0
committed chunks: 39
buffered files: 8, buffered chunks: 8
flush count: 1
model: gpt-4o-mini
input tokens: 14081 (0.001$), output tokens: 1327 (0.000$)
```

`rag build` takes very long time and money (if you're using a proprietary api). It creates chunks and add title and summary to each chunk, using AI.

You can press Ctrl+C to pause the process. You can resume from where you left off by running `rag build` again. (more on [a dedicated document](./commands/build.txt))

```
ai_tutorials/
  |
  *-- .ragit/
  |   |
  |   *-- chunks/
  |   |   |
  |   |   *-- ... a lot of directories
  |   |
  |   *-- configs/
  |   |
  |   *-- files/
  |   |
  |   *-- images/
  |   |
  |   *-- prompts/
  |   |
  |   *-- index.json
  |   |
  |   *-- models.json
  |
  *-- ai_tutorial_1.txt
  |
  *-- ai_tutorial_2.txt
  |
  *-- ai_tutorial_3.txt
  |
  *-- ... and many more txt files
```

After it's built, you'll see many data files in the `.ragit/` directory. You can ask queries on the knowledge-base now.

NOTE: You can ask queries on an incomplete knowledge-base, too.

## 2. Clone Knowledge-Bases from web

This is the key part. You can download knowledge-bases from the internet. You can also share your knowledge-base with others.

I have uploaded some sample knowledge-bases to [https://ragit.baehyunsol.com](https://ragit.baehyunsol.com). You can clone one like `rag clone https://ragit.baehyunsol.com/sample/ragit`. This is a knowledge-base of ragit's documents.

## 3. Change Configs

Before asking a question or building a knowledge-base, you may want to change configurations. Configurations are very important because most commands cost money and you can optimize it with proper configurations.

### Per Knowledge-Base Configuration

Run `rag config --get model`. You'll see which model is used to answer your queries and build a knowledge-base.

Let's say you have free credits for Anthropic. By running `rag config --set model claude-3.5-sonnet`, you can change your default model.

Run `rag config --get-all` to see all the keys and values.

### Global Configuration

If you want to set default configurations for all your repositories, you can create configuration files in `~/.config/ragit/`:

- `~/.config/ragit/api.json` - For API configuration (model, timeout, etc.)
- `~/.config/ragit/build.json` - For build configuration (chunk size, etc.)
- `~/.config/ragit/query.json` - For query configuration (max titles, etc.)

These files can contain just the specific fields you want to override - you don't need to include all configuration options. For example, if you only want to set a default model, your `~/.config/ragit/api.json` could be as simple as:

```json
{
  "model": "claude-3.5-sonnet"
}
```

When you run `rag init` to create a new repository, these global configurations will be used as defaults. This is especially useful if you always want to use a specific model or have specific build parameters.

## 4. Ask questions on a Knowledge-Base

Asking query is straight forward: `rag query "Tell me how the rust compiler uses git"`

If you want an interactive chat, run `rag query --interactive`.
