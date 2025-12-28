function fillExample() {
    document.getElementById('from').value = '0xalice11111111111111111111111111111111111';
    document.getElementById('to').value = '0xbob222222222222222222222222222222222222';
    document.getElementById('amount').value = '100';
    document.getElementById('fee').value = '1';
}

async function updateStatus() {
    try {
        const response = await fetch('/api/status');
        const data = await response.json();

        document.getElementById('height').textContent = data.height;
        document.getElementById('mempool').textContent = data.mempool_size;
        document.getElementById('difficulty').textContent = data.difficulty;
    } catch (error) {
        console.error('Failed to fetch status', error);
    }
} 

document.getElementById('tx-form').addEventListener('submit', async(e) => {
    e.preventDefault();

    const messageE1 = document.getElementById('message')
    const sumbitBtn = document.getElementById('submit-btn');
    messageE1.style.display = 'none';
    sumbitBtn.disabled = true;
    sumbitBtn.textContent = ' Submitting ...';

    const transaction = {
        from: document.getElementById('from').value,
        to: document.getElementById('to').value,
        amount: parseInt(document.getElementById('amount').value),
        fee: parseInt(document.getElementById('fee').value),
    };

    try {
        const response = await fetch('api/transaction', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(transaction),
        });

        const result = await response.json();

        messageE1.textContent = result.message;
        messageE1.className = 'message ' + (result.success ? 'success' : 'error');
        messageE1.style.display = 'block';

        if (result.success) {
            updateStatus();
        }
    } catch(error) {
        messageE1.textContent = 'Failed to submit transaction: ' + error.message;
        messageE1.className = 'message error';
        messageE1.style.display = 'block';
    } finally {
        sumbitBtn.disabled = false;
        sumbitBtn.textContent = 'Submit Transaction';
    }

});

updateStatus();

setInterval(updateStatus, 2000);