const pkInput = document.getElementById("pk_input")
const pkAddButton = document.getElementById("pk_add_button")
const pkList = document.getElementById("pk_list")
const ToList = document.getElementById("to_list")
const submit = document.getElementById('submit')
const fee = document.getElementById('fee')

function clear() {
    ToList.innerHTML = ''
    fee.value = ''
}

let pks = [];
let tos = [];
const Addr = '0211d337ed116a694083637bb1f20a57e97295c8e0958323a1ea2f468fe52b1ee5'

function renderPkList() {
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

        const sendBtn = document.createElement('button')
        sendBtn.className = 'send_button'
        sendBtn.textContent = 'Send'
        sendBtn.onclick = () => amountPopup(index)

        li.appendChild(span)
        li.appendChild(deleteBtn)
        li.appendChild(sendBtn)
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
    renderPkList()
}

function deletePk(index){
    pks.splice(index, 1)
    renderPkList()
}

function amountPopup(index) {
    const userInput = prompt("Enter amount: ");
    const amount = parseInt(userInput);

    if(!isNaN(amount)){
        addTo(index, amount)
    }else{
        alert("That's not a valid number")
    }
}

function addTo(index, amount){
    tos.push([pks[index], amount])
    renderToList()
}

function deleteTo(index){
    tos.splice(index, 1)
    renderToList()
}

function renderToList(){
    ToList.innerHTML = '';

    if (tos.length === 0){
        ToList.innerHTML = '<li class="empty">No public keys yet</li>'
        return;
    }

    tos.forEach(([pk, amount], index) => {
        const li = 
        document.createElement('li');
        li.className = 'pkto'
        const span = document.createElement('span')
        span.textContent =  `${pk}: ${amount}`
        
        const deleteBtn = document.createElement('button')
        deleteBtn.className = 'remove_button';
        deleteBtn.textContent = 'Remove';
        deleteBtn.onclick = () => deleteTo(index)

        li.appendChild(span)
        li.appendChild(deleteBtn)
        ToList.appendChild(li)
    })     


}

pkAddButton.addEventListener('click', addPk)
pkInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        addPk()
    }
})

submit.addEventListener('click',  async () =>{
    console.log('Submit button clicked!');  //
    const feeValue = parseInt(fee.value, 10);
    const transaction = {
        to: tos.map(item => item[0]),
        to_amount: tos.map(item => item[1]),
        fee: feeValue
    };

    try{
        const response = await fetch('api/transaction', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(transaction),
        });

        console.log('Response status:', response.status);
        console.log('Response headers:', response.headers.get('content-type'));
        
        // Get the raw text first to see what we're getting
        const text = await response.text();
        console.log('Raw response:', text);
        
        // Now try to parse it
        const result = JSON.parse(text);
        console.log('Parsed result:', result);
    } catch(error){
        console.error('Caught error:', error);
    } finally {
        clear()
    }

});

renderPkList()
renderToList()