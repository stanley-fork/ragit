# It fetches ai-model-list from model-store of ragit.baehyunsol.com, and updates
# `models.json` in the repository. I'm doing this because I don't want to have
# 2 separate source of truth.
#
# I had 2 choices:
# 1. Manually update `models.json` in the repository and automatically update
#    model-store according to the file.
# 2. Manually update model-store and automatically update `models.json` in
#    the repository.
#
# I chose #2 because, in order to implement #1, I need a bot that regularly
# checks the repository and updates the model-store, and I don't want to
# implement another bot. I prefer doing it manually: update model-store ->
# run this script -> git commit & push

import json
import os
from server import get_json
from utils import goto_root

paths = [
    "./models.json",
    "./ragithub/backend/models.json",
    "./crates/api/models.json",
]

goto_root()

for path in paths:
    assert os.path.exists(path)

offset = 0
models = []

while True:
    models_ = get_json(
        url=f"https://ragit.baehyunsol.com/ai-model-list",
        raw_url=True,
        query={
            "limit": 50,
            "offset": offset,
        },
    )
    offset += 50
    models += models_

    if len(models_) < 50:
        break

models = [
    {
        # The original models.json is sorted in this order and I like this order.
        k: model[k] for k in [
            "name",
            "api_name",
            "can_read_images",
            "api_provider",
            "api_url",
            "input_price",
            "output_price",
            "explanation",
            "tags",
            "api_env_var",
        ]
    } for model in models
]

# Models are sorted by api_env_var because a group of models share api_env_var.
models.sort(key=lambda m: m["name"])  # break tie
models.sort(key=lambda m: (m["api_env_var"] or "ZZZ"))

for path in paths:
    with open(path, "w") as f:
        f.write(json.dumps(models, indent=4))
