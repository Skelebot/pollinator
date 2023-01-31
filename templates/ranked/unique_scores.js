let radios = new Array(max);
let checks = new Array(max);
update_radios();

function update_radios() {
    radios = new Array(max);
    checks = new Array(max);
    for (let y = 0; y < max; y++) {
        radios[y] = new Array(x_max);
        for (let x = 0; x < x_max; x++) {
            radios[y][x] = document.getElementById(`${y}_${x}`);
            if (radios[y][x].checked) checks[y] = x;
            radios[y][x].onclick = swap_with;
        }
    }
}

function swap_with(event) {
    let y = parseInt(event.target.name);
    let x = parseInt(event.target.value);

    if (can_unranked) {
        if (x === max) {
            checks[y] = max;
            return;
        }
    }

    let prev_x = checks[y];
    checks[y] = x;
    for (let iy = 0; iy < max; iy++) {
        if (iy === y) { continue; }
        if (radios[iy][x].checked) {
            radios[iy][x].checked = false;
            radios[iy][prev_x].checked = true;
            checks[iy] = prev_x;
        }
    }
}