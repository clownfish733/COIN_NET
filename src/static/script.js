const pkInput = document.getElementById("pk_input")
const pkAddButton = document.getElementById("pk_add_button")
const pkList = document.getElementById("pk_list")

let pks = [];

function renderList() {
    pkList.innerHTML = '';

    if (pks.length === 0){
        pkList.innerHTML = '<li class="empty">No public keys yet</li>'
        return;
    }

    pks.forEach((pk, index) => {
        const li = 
        document.createElement('li');
        li.className = 'pk'

        const span = document.createElement('span')
        span.textContent = pk

        const deleteBtn = document.createElement('button')
        deleteBtn.className = 'delete_button';
        deleteBtn.textContent = 'Delete';
        deleteBtn.onclick = () => deletePk(index)
        li.appendChild(span)
        li.appendChild(deleteBtn)
        pkList.appendChild(li)
    })
}

function addPk() {
    const value = pkInput.value.trim()

    if (value === ''){
        alert('please enter a value!')
        return
    }
    pks.push(value)
    pkInput.value = ''
    pkInput.focus()
    renderList()
}

function deletePk(index){
    pks.splice(index, 1)
    renderList()
}

pkAddButton.addEventListener('click', addPk)
pkInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        addPk()
    }
})

renderList()