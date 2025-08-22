let deactivated_elements = document.querySelectorAll(".deactivated");
let activated_elements = document.querySelectorAll(".activated");
let button_box = document.querySelector(".toggle-buttons");

function toggle_header() {
    for (let i = 0; i < deactivated_elements.length; i++) {
        deactivated_elements[i].classList.toggle("activated");
        deactivated_elements[i].classList.toggle("deactivated");
    }

    for (let i = 0; i < activated_elements.length; i++) {
        activated_elements[i].classList.toggle("activated");
        activated_elements[i].classList.toggle("deactivated");
    }

    if (button_box.style.maxHeight) {
        button_box.style.maxHeight = null;
    }

    else {
        button_box.style.maxHeight = button_box.scrollHeight + "px";
    }
}

document.getElementById("header-button").addEventListener("click", toggle_header)
