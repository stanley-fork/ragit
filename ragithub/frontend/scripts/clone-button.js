var clone_info = document.getElementById("clone-info");
var clone_button = document.getElementById("clone-button");
var is_clone_info_visible = false;

function toggle_clone_info() {
  if (is_clone_info_visible) {
    clone_info.style.display = "none";
    clone_button.style.backgroundColor = "var(--black)";
    clone_button.style.color = "var(--white)";
    is_clone_info_visible = false;
  } else {
    clone_info.style.display = "flex";
    clone_button.style.backgroundColor = "var(--white)";
    clone_button.style.color = "var(--black)";
    is_clone_info_visible = true;
  }
}
