from subprocess import TimeoutExpired
import time
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

# Found a bug in ragit!
# 1. Main process spawns child processes and ask them to create chunks.
# 2. Main process is killed, but the children don't know that. They just keep creating chunks.
# 3. Next `rag build` spawns another main process. It runs `rag check --recover` before it starts.
# 4. Children from 2 finish creating chunks after `rag check --recover`.
# 5. `rag check --recover` from step 3 is supposed to remove the chunks created by the children from 2, but it doesn't.
# 6. `rag build` from step 3 creates another chunk for the same file. Now ragit doesn't know which chunk is from step 2 and which is from step 3.
def orphan_process(test_model: str):
    for i in range(2):
        goto_root()
        mk_and_cd_tmp_dir()
        cargo_run(["init"])

        for i in range(15):
            write_string(f"sample{i}.txt", "Hello, world!")
            cargo_run(["add", f"sample{i}.txt"])

        # NOTE: we cannot use dummy models here: they always create the same chunk, with the same uids,
        #       so the chunks from step 2 overwrites chunks from step 1
        # step 1: initialize workers and kill their parent before they finish their job
        cargo_run(["config", "--set", "model", test_model])
        cargo_run(["config", "--set", "sleep_after_llm_call", "20000"])

        try:
            # NOTE: workers must finish creating a chunk in 6 seconds. please use a fast model
            cargo_run(["build", "--jobs=4"], timeout=26)

        except TimeoutExpired:
            pass

        # step 2: initialize workers again
        cargo_run(["config", "--set", "sleep_after_llm_call", "0"])
        cargo_run(["build"])

        # step 3: let workers (from step 1) cook remaining chunks
        time.sleep(20)

        # step 4: mix chunks from step 2 and step 3
        # its behavior differs when there's `rag check --recover`
        if i == 0:
            cargo_run(["check", "--recover"])

        cargo_run(["check"])
