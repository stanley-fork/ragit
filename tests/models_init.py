import json
import os
import shutil
from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

def models_init():
    """Test the initialization of models.json and model selection in api.json."""
    goto_root()
    
    # Test with default model
    print("\n--- Testing with default model ---")
    mk_and_cd_tmp_dir()
    
    # Run the ragit init command
    print("Running 'rag init'...")
    cargo_run(["init"])
    
    # Check if the models.json file exists
    models_json_path = os.path.join(".ragit", "models.json")
    assert os.path.exists(models_json_path), f"models.json file does not exist at: {models_json_path}"
    print(f"models.json file exists at: {models_json_path}")
    
    # Read the content of the models.json file
    with open(models_json_path, 'r') as f:
        models_json = json.load(f)
    
    print(f"Number of models in models.json: {len(models_json)}")
    model_names = [model['name'] for model in models_json]
    print(f"Model names: {model_names}")
    
    # Check if the api.json file exists
    api_json_path = os.path.join(".ragit", "configs", "api.json")
    assert os.path.exists(api_json_path), f"api.json file does not exist at: {api_json_path}"
    print(f"api.json file exists at: {api_json_path}")
    
    # Read the content of the api.json file
    with open(api_json_path, 'r') as f:
        api_json = json.load(f)
    
    selected_model = api_json.get('model', 'Not found')
    print(f"Selected model in api.json: {selected_model}")
    
    # Check if the selected model exists in models.json
    assert selected_model in model_names, f"Selected model '{selected_model}' does NOT exist in models.json"
    print(f"Selected model '{selected_model}' exists in models.json")
    
    # Clean up the previous test
    os.chdir("..")
    
    # Test with custom models.json without the default model
    print("\n--- Testing with custom models.json without default model ---")
    mk_and_cd_tmp_dir()
    
    # Create a custom models.json file without the default model
    custom_models = [
        {
            "name": "gpt-4o",
            "api_name": "gpt-4o",
            "can_read_images": True,
            "api_provider": "openai",
            "api_url": "https://api.openai.com/v1/chat/completions",
            "input_price": 2.5,
            "output_price": 10.0,
            "api_timeout": None,
            "explanation": None,
            "api_key": None,
            "api_env_var": "OPENAI_API_KEY"
        },
        {
            "name": "gpt-4o-mini",
            "api_name": "gpt-4o-mini",
            "can_read_images": True,
            "api_provider": "openai",
            "api_url": "https://api.openai.com/v1/chat/completions",
            "input_price": 0.15,
            "output_price": 0.6,
            "api_timeout": None,
            "explanation": None,
            "api_key": None,
            "api_env_var": "OPENAI_API_KEY"
        },
        {
            "name": "phi-4-14b-ollama",
            "api_name": "phi4:14b",
            "can_read_images": True,
            "api_provider": "openai",
            "api_url": "http://127.0.0.1:11434/v1/chat/completions",
            "input_price": 0.0,
            "output_price": 0.0,
            "api_timeout": None,
            "explanation": None,
            "api_key": None,
            "api_env_var": None
        }
    ]
    
    # Create a temporary models.json file
    custom_models_path = os.path.join(os.getcwd(), "custom_models.json")
    with open(custom_models_path, 'w') as f:
        json.dump(custom_models, f, indent=2)
    
    # Run the ragit init command with the custom models.json file
    print("Running 'rag init' with custom models.json...")
    os.environ["RAGIT_MODEL_CONFIG"] = custom_models_path
    cargo_run(["init"])
    
    # Check if the models.json file exists
    models_json_path = os.path.join(".ragit", "models.json")
    assert os.path.exists(models_json_path), f"models.json file does not exist at: {models_json_path}"
    print(f"models.json file exists at: {models_json_path}")
    
    # Read the content of the models.json file
    with open(models_json_path, 'r') as f:
        models_json = json.load(f)
    
    print(f"Number of models in models.json: {len(models_json)}")
    model_names = [model['name'] for model in models_json]
    print(f"Model names: {model_names}")
    assert len(models_json) == 3, f"Expected 3 models, got {len(models_json)}"
    
    # Check if the api.json file exists
    api_json_path = os.path.join(".ragit", "configs", "api.json")
    assert os.path.exists(api_json_path), f"api.json file does not exist at: {api_json_path}"
    print(f"api.json file exists at: {api_json_path}")
    
    # Read the content of the api.json file
    with open(api_json_path, 'r') as f:
        api_json = json.load(f)
    
    selected_model = api_json.get('model', 'Not found')
    print(f"Selected model in api.json: {selected_model}")
    
    # Check if the selected model exists in models.json
    assert selected_model in model_names, f"Selected model '{selected_model}' does NOT exist in models.json"
    print(f"Selected model '{selected_model}' exists in models.json")
    
    # Check if it's the lowest-cost model (phi-4-14b-ollama)
    assert selected_model == 'phi-4-14b-ollama', f"Expected lowest-cost model 'phi-4-14b-ollama', got '{selected_model}'"
    print("SUCCESS: The lowest-cost model was correctly selected!")
    
    # Clean up environment variable
    if "RAGIT_MODEL_CONFIG" in os.environ:
        del os.environ["RAGIT_MODEL_CONFIG"]

if __name__ == "__main__":
    models_init()
