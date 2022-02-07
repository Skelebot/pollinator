options_list.forEach((element, index) => {
    // generates a poll preview from a string array `options_list`
    var div = document.createElement("div");
    div.className = "poll_option";

    var option = document.createElement("input");
    option.type = input_type;
    option.width = "20px";

    option.value = "opt" + index;
    option.name = "poll_preview";
    div.appendChild(option);

    var label = document.createElement("label");
    label.setAttribute("for", "opt" + index);
    label.textContent = element;
    div.appendChild(label);
    options.appendChild(div);
});
