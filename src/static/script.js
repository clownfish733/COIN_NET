const AddressInput = document.getElementById("address_input")
const AddressLabel = document.getElementById("address_label")
const AddressAddButton = document.getElementById("address_add_button")
const AddressBook = document.getElementById("address-list")
const RecipientList = document.getElementById("recipient-list")
const submit = document.getElementById('submit')
const fee = document.getElementById('fee')

let address_book = new Map(); //label : address
let recipients = []; //label: amount

function clear_transaction(){
    RecipientList.innerHTML = ''
    fee.value = ''
}

//Address Book Handling -------------------------------------------------------

function renderAddressBook() {
    AddressBook.innerHTML = '';
    if (address_book.length === 0){
        address_book.innerHTML = '';
        return;
    }

    address_book.forEach((_address, label) => {
        const li = 
        document.createElement('li');
        li.className = 'address'

        const span = document.createElement('span')
        span.textContent = label
        span.className = "address-label"

        const deleteBtn = document.createElement('button')
        deleteBtn.className = 'delete_button';
        deleteBtn.textContent = 'Delete';
        deleteBtn.onclick = () => deleteAddress(label)

        const sendBtn = document.createElement('button')
        sendBtn.className = 'send_button'
        sendBtn.textContent = 'Send'
        sendBtn.onclick = () => amountPopup(label)

        li.appendChild(span)
        li.appendChild(sendBtn)
        li.appendChild(deleteBtn)
        AddressBook.appendChild(li)
    })
}



function addAddress() {
    const address = AddressInput.value.trim()
    const label = AddressLabel.value.trim()

    if (address === ''){
        alert('please enter an address!')
        return
    }
    if (label === ''){
        alert('please enter a label')
        return
    }
    if (address_book.has(label)){
        alert('please enter unique label')
        return
    }
    address_book.set(label, address);
    AddressInput.value = ''
    AddressLabel.value = ''

    AddressInput.focus()
    AddressLabel.focus()
    renderAddressBook()
}

function deleteAddress(label){
    address_book.delete(label)
    renderAddressBook()
}

AddressAddButton.addEventListener('click', addAddress)
AddressInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        addAddress
    }
})

//recipients handling -------------------------------------------------------------------

function renderRecipients(){
    RecipientList.innerHTML = '';

    if (recipients.length === 0){
        RecipientList.style.display = 'none'
        return
    }
    RecipientList.style.display = ''
    

    recipients.forEach(([recipient, amount], index) => {
        const li = document.createElement('li');
        li.className = 'Outputs'
        const div = document.createElement('div');
        div.className = 'output-card'
        
        const span_recipient = document.createElement('span')
        span_recipient.className = 'recipient'
        span_recipient.textContent =  `${recipient}`
        
        const span_amount = document.createElement('span')
        span_amount.className = "output-amount"
        span_amount.textContent = `${amount}`
        
        

        const deleteBtn = document.createElement('button')
        deleteBtn.className = 'remove-button';
        deleteBtn.textContent = 'Remove';
        deleteBtn.onclick = () => deleteRecipient(index)

        div.appendChild(span_recipient);
        div.appendChild(span_amount);
        div.appendChild(deleteBtn);

        li.appendChild(div)
        RecipientList.appendChild(li)
    })     
}

function addRecipient(label, amount){
    recipients.push([label, amount])
    renderRecipients()
}

function deleteRecipient(index){
    recipients.splice(index, 1)
    renderRecipients()
}

function amountPopup(label) {
    const userInput = prompt("Enter amount: ");
    const amount = parseInt(userInput);

    if(!isNaN(amount)){
        addRecipient(label, amount)
    }else{
        alert("That's not a valid number")
    }
}

//api handling ----------------------------------------------------------------------

async function loadAddressBook() {
    try{
        console.log('Loading address book');
        const response = await fetch('/api/address_book');
        const data = await response.json()
        address_book = new Map(Object.entries(data));
    } catch(err) {
        console.error('Failed to load Address Book:', err);
        address_book = M
    }
    renderAddressBook()
}

async function saveAddressBook() {
    try{
        const obj = Object.fromEntries(address_book);

        await fetch('/api/address_book', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json'},
            body: JSON.stringify(obj)
        });
        console.log('Addresss Book saved')
    } catch(err) {
        console.error("failed to save Address Book:", err);
    }
}

async function  updateStatus() {
    try {
        const response = await fetch('/api/node_status');
        const data = await response.json();
        document.getElementById("height").textContent = data.height
        document.getElementById("mempool").textContent = data.mempool_size
        document.getElementById("difficulty").textContent = data.difficulty
    } catch(error) {
        console.error("Failed to fetch node status", error);
    }
    try{
        const response = await fetch('/api/user_status');
        const data = await response.json();
        document.getElementById('user-address').textContent = data.pk
        document.getElementById('funds').textContent = data.amount
    } catch(error) {
        console.error("Failed to fetch user status")
    }
}

submit.addEventListener('click',  async () =>{
    console.log('Submit button clicked!');  //
    const feeValue = parseInt(fee.value, 10);
    const transaction = {
        to: recipients.map(item => address_book.get(item[0])),
        to_amount: recipients.map(item => item[1]),
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
        clear_transaction()
        const text = await response.text();
        
        const result = JSON.parse(text);
        document.getElementById('message').textContent  = result.message;
    
        document.getElementById('message').className = (result.success ? 'success' : 'fail')
        document.getElementById('message').style.display = 'block'

        updateStatus()

    } catch(error){
        console.error('Caught error:', error);
    } finally {
        transaction
    }

});

setInterval(async() => {
    try{
        const response = await fetch('/api/save_check');
        const data = await response.json();

        if (data.save) {
            await saveAddressBook()
            document.body.innerHTML = '<h1>Server has shut down. Please close this tab.</h1>';
        }
    } catch (err) {

    }
})
setInterval(updateStatus, 2000);

//Initial function calls---------------------------------------------------------------------
renderAddressBook()
renderRecipients()
updateStatus()

document.addEventListener('DOMContentLoaded', loadAddressBook);