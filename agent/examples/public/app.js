document.addEventListener('DOMContentLoaded', () => {
    const chatHistory = document.getElementById('chatHistory');
    const userInput = document.getElementById('userInput');
    const sendButton = document.getElementById('sendButton');
    const statusIndicator = document.getElementById('statusIndicator');
    const approvalValue = document.getElementById('approvalValue');
    const approvalBanner = document.getElementById('approvalBanner');
    
    // API endpoint
    const API_URL = '/agent/execute';
    
    // Function to update status indicator
    function updateStatus(status) {
        statusIndicator.className = 'status-indicator ' + status;
        const statusText = statusIndicator.querySelector('.status-text');
        
        switch(status) {
            case 'idle':
                statusText.textContent = 'Idle';
                break;
            case 'thinking':
                statusText.textContent = 'Processing...';
                break;
            case 'error':
                statusText.textContent = 'Error';
                break;
        }
    }
    
    // Function to add a message to the chat history
    function addMessage(content, isUser = false) {
        const messageDiv = document.createElement('div');
        messageDiv.className = `message ${isUser ? 'user-message' : 'agent-message'}`;
        
        const messageContent = document.createElement('div');
        messageContent.className = 'message-content';
        
        // If it's an agent message, we might need to handle markdown-like formatting
        if (!isUser && typeof content === 'object') {
            // Handle structured response from the agent
            if (content.result && content.result.response) {
                messageContent.innerHTML = formatAgentResponse(content.result.response);
            } else {
                messageContent.innerHTML = formatAgentResponse(JSON.stringify(content, null, 2));
            }
        } else {
            messageContent.textContent = content;
        }
        
        messageDiv.appendChild(messageContent);
        chatHistory.appendChild(messageDiv);
        
        // Scroll to the bottom of the chat history
        chatHistory.scrollTop = chatHistory.scrollHeight;
    }
    
    // Function to format agent responses (simple markdown-like formatting)
    function formatAgentResponse(text) {
        if (!text) return '';
        
        // Convert markdown-style code blocks
        text = text.replace(/```([\s\S]*?)```/g, '<pre><code>$1</code></pre>');
        
        // Convert markdown-style inline code
        text = text.replace(/`([^`]+)`/g, '<code>$1</code>');
        
        // Convert line breaks to <br>
        text = text.replace(/\n/g, '<br>');
        
        return text;
    }
    
    // Function to update approval state
    function updateApprovalState(state) {
        if (state && state.approved) {
            approvalValue.textContent = 'APPROVED';
            approvalValue.classList.add('approved');
            approvalBanner.classList.add('visible');
            
            // Scroll to top to make sure banner is visible
            window.scrollTo(0, 0);
        }
    }
    
    // Function to send a message to the agent
    async function sendMessage() {
        const message = userInput.value.trim();
        if (!message) return;
        
        // Add user message to chat
        addMessage(message, true);
        
        // Clear input and disable send button
        userInput.value = '';
        sendButton.disabled = true;
        updateStatus('thinking');
        
        try {
            const response = await fetch(API_URL, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ task: message })
            });
            
            const data = await response.json();
            
            if (response.ok) {
                // Add agent response to chat
                addMessage(data.result.response);
                updateStatus('idle');
                
                // Check and update approval state
                if (data.state) {
                    updateApprovalState(data.state);
                }
            } else {
                // Handle error
                addMessage(`Error: ${data.error || 'Something went wrong'}`, false);
                updateStatus('error');
                setTimeout(() => updateStatus('idle'), 3000);
            }
        } catch (error) {
            console.error('Error:', error);
            addMessage(`Error: Could not connect to the agent. Please try again later.`, false);
            updateStatus('error');
            setTimeout(() => updateStatus('idle'), 3000);
        }
        
        // Re-enable send button
        sendButton.disabled = false;
    }
    
    // Event listeners
    sendButton.addEventListener('click', sendMessage);
    
    userInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    });
    
    // Focus input on page load
    userInput.focus();
});
