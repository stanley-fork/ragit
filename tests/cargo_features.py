import json
from utils import cargo_run

def cargo_features():
    combinations = [
        { "csv": False, "korean": False, "pdf": False, "svg": False },
        { "csv": False, "korean": False, "pdf": False, "svg": True },
        { "csv": False, "korean": False, "pdf": True, "svg": False },
        { "csv": False, "korean": False, "pdf": True, "svg": True },
        { "csv": False, "korean": True, "pdf": False, "svg": False },
        { "csv": False, "korean": True, "pdf": False, "svg": True },
        { "csv": False, "korean": True, "pdf": True, "svg": False },
        { "csv": False, "korean": True, "pdf": True, "svg": True },
        { "csv": True, "korean": False, "pdf": False, "svg": False },
        { "csv": True, "korean": False, "pdf": False, "svg": True },
        { "csv": True, "korean": False, "pdf": True, "svg": False },
        { "csv": True, "korean": False, "pdf": True, "svg": True },
        { "csv": True, "korean": True, "pdf": False, "svg": False },
        { "csv": True, "korean": True, "pdf": False, "svg": True },
        { "csv": True, "korean": True, "pdf": True, "svg": False },
        { "csv": True, "korean": True, "pdf": True, "svg": True },
    ]

    for combination in combinations:
        build_options = json.loads(cargo_run(
            ["version", "--build-options", "--json"],
            features=[feature for feature, enabled in combination.items() if enabled],
            stdout=True,
        ))
        assert len(build_options["features"]) == len(combination)

        for feature, enabled in combination.items():
            assert build_options["features"][feature] == enabled
