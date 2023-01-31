// generates a poll preview from a string array `options_list`

var opts_num = options_list.length;
let table = document.createElement("table");
let tr = document.createElement("tr");
for(var num=0; num < opts_num; num++) {
    let th = document.createElement("th");
    th.innerHTML = num + 1;
    tr.appendChild(th);
}
if (can_unranked) {
    let th = document.createElement("th");
    th.innerHTML = "-";
    tr.appendChild(th);
}
table.appendChild(tr);

for(var optn=0; optn < opts_num; optn++) {
    let otr = document.createElement("tr");

    for(var opti=0; opti < opts_num; opti++) {
        let inputtd = document.createElement("td");
        let input = document.createElement("input");
        input.type = "radio";
        input.id = optn + "_" + opti;
        input.value = opti;
        input.name = optn;
        if (!can_unranked && opti==optn) {
            input.checked = true;
        }
        inputtd.appendChild(input);
        otr.appendChild(inputtd);
    }
    if (can_unranked) {
        let inputtd = document.createElement("td");
        let input = document.createElement("input");
        input.type = "radio";
        input.id = optn + "_" + opts_num;
        input.value = opts_num;
        input.name = optn;
        input.checked = true;
        inputtd.appendChild(input);
        otr.appendChild(inputtd);
    }
    let labeltd = document.createElement("td");
    let label = document.createElement("label");
    label.for = "opt" + optn;
    label.innerHTML = options_list[optn];
    labeltd.appendChild(label);
    otr.appendChild(labeltd);

    table.appendChild(otr);
}

options.appendChild(table);
