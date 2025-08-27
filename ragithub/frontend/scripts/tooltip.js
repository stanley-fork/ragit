"use strict";

let tooltips = document.querySelectorAll(".tooltip-container");

for (let i = 0; i < tooltips.length; i++) {
    let child = document.getElementById("tooltip-message-" + i);

    document.getElementById("tooltip-container-" + i).addEventListener("mousemove", e => {
        if (e.clientX + child.clientWidth > window.innerWidth) {
            child.style.left = e.clientX - child.clientWidth + "px";
        }

        else {
            child.style.left = e.clientX + "px";
        }

        if (e.clientY < child.clientHeight + 8) {
            child.style.top = e.clientY + 8 + "px";
        }

        else {
            child.style.top = (e.clientY - child.clientHeight - 8) + "px";
        }
    });
}
