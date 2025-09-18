from server import (
    create_user,
    delete_json,
    get_api_key,
    get_json,
    put_json,
    spawn_ragit_server,
)
from utils import deepcopy

def server_ai_model():
    server_process = None

    try:
        server_process = spawn_ragit_server()
        create_user(id="test-user", password="12345678")
        admin_api_key = get_api_key(id="test-user", password="12345678")
        ai_models1 = get_json(url="http://127.0.0.1:41127/ai-model-list", raw_url=True)
        ai_models1 = remove_timestamp(ai_models1)

        new_model1 = deepcopy(ai_models1[0])
        new_model1["explanation"] = "this is a new model!!"
        new_model1 = remove_timestamp(new_model1)
        new_model2 = deepcopy(ai_models1[1])
        new_model2["name"] = "new-name"
        new_model2 = remove_timestamp(new_model2)

        # test 1: only admin can upload models
        put_json(
            url="http://127.0.0.1:41127/ai-model-list",
            body=new_model1,
            raw_url=True,
            api_key=None,
            expected_status_code=403,
        )

        # test 2: update an existing model
        put_json(
            url="http://127.0.0.1:41127/ai-model-list",
            body=new_model1,
            raw_url=True,
            api_key=admin_api_key,
        )
        ai_models2 = get_json(url="http://127.0.0.1:41127/ai-model-list", raw_url=True)
        ai_models2 = remove_timestamp(ai_models2)
        assert len(ai_models1) == len(ai_models2)
        assert new_model1 not in ai_models1
        assert new_model1 in ai_models2

        # test 3: upload a new model
        put_json(
            url="http://127.0.0.1:41127/ai-model-list",
            body=new_model2,
            raw_url=True,
            api_key=admin_api_key,
        )
        ai_models3 = get_json(url="http://127.0.0.1:41127/ai-model-list", raw_url=True)
        ai_models3 = remove_timestamp(ai_models3)
        assert len(ai_models1) + 1 == len(ai_models3)
        assert new_model2 not in ai_models1
        assert new_model2 not in ai_models3

        # check if we can get a model by name
        assert get_json(
            url=f"http://127.0.0.1:41127/ai-model-list?name={new_model2['name']}",
            raw_url=True,
        )[0]["explanation"] == new_model2["explanation"]

        # test 4: only admin can delete a model
        delete_json(
            url=f"http://127.0.0.1:41127/ai-model-list/{new_model1['name']}",
            raw_url=True,
            api_key=None,
            expected_status_code=403,
        )

        # test 5: delete a model
        delete_json(
            url=f"http://127.0.0.1:41127/ai-model-list/{new_model1['name']}",
            raw_url=True,
            api_key=admin_api_key,
        )
        assert len(get_json(
            url=f"http://127.0.0.1:41127/ai-model-list?name={new_model1['name']}",
            raw_url=True,
        )) == 0

        # test 6: since it's already deleted, we cannot delete this again
        delete_json(
            url=f"http://127.0.0.1:41127/ai-model-list/{new_model1['name']}",
            raw_url=True,
            api_key=admin_api_key,
            expected_status_code=404,
        )

    finally:
        if server_process is not None:
            server_process.kill()

def remove_timestamp(v):
    if isinstance(v, list):
        return [remove_timestamp(e) for e in v]

    else:
        v = deepcopy(v)
        v.pop("created_at", None)
        v.pop("updated_at", None)
        return v
