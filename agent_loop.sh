#!/usr/bin/env bash

# Ensure we are in the repository directory
cd "$(dirname "$0")"

LOG_FILE="agent_loop.log"
STATE_FILE="agent_loop.state"

if [ -z "$1" ]; then
    echo "Usage: $0 <num_iterations>" | tee -a "$LOG_FILE"
    echo "Example: $0 3" | tee -a "$LOG_FILE"
    exit 1
fi

ITERATIONS=$1

# Default state
START_ITERATION=1
START_PHASE=1

# Load state if it exists
if [ -f "$STATE_FILE" ]; then
    source "$STATE_FILE"
    echo "Resuming from iteration $START_ITERATION, phase $START_PHASE" | tee -a "$LOG_FILE"
fi

echo "Running agy loop for $ITERATIONS iterations..." | tee -a "$LOG_FILE"

save_state() {
    local it=$1
    local ph=$2
    echo "START_ITERATION=$it" > "$STATE_FILE"
    echo "START_PHASE=$ph" >> "$STATE_FILE"
}

# Run the command with retry logic for proot stability
run_agy_with_retry() {
    local max_retries=3
    local retry=0
    local success=0
    
    while [ $retry -lt $max_retries ]; do
        # Execute the command and pipe output to tee
        if "$@" 2>&1 | tee -a "$LOG_FILE"; then
            success=1
            break
        else
            echo "Command failed with exit code $?. Retrying... ($((retry + 1))/$max_retries)" | tee -a "$LOG_FILE"
            retry=$((retry + 1))
            sleep 5
        fi
    done
    
    if [ $success -eq 0 ]; then
        echo "Command failed after $max_retries attempts. Exiting so it can be resumed later." | tee -a "$LOG_FILE"
        exit 1
    fi
}

for (( i=START_ITERATION; i<=ITERATIONS; i++ )); do
    echo "==================================================" | tee -a "$LOG_FILE"
    echo " Starting iteration $i of $ITERATIONS" | tee -a "$LOG_FILE"
    echo "==================================================" | tee -a "$LOG_FILE"
    
    if [ "$START_PHASE" -le 1 ]; then
        save_state "$i" 1
        echo "--- Phase 1: Issue Discovery and Refinement ---" | tee -a "$LOG_FILE"
        run_agy_with_retry agy --dangerously-skip-permissions --print-timeout 1h --print "Find the next unblocked issue in GitHub (use the gh cli tool and find the repo splash)  labeled 'ready-for-agent'. Read the issue description and refine the requirements. You are running in autonomous mode so do not create an artifact or ask for approval on a document, simply output requiremnets for the next agent to read." < /dev/null
    fi
    
    if [ "$START_PHASE" -le 2 ]; then
        save_state "$i" 2
        echo "--- Phase 2: TDD ---" | tee -a "$LOG_FILE"
        run_agy_with_retry agy --dangerously-skip-permissions --print-timeout 1h -c --print "Now implement the requirements for the issue using the tdd skill at /root/splash/.agents/skills/tdd/SKILL.md. You are running autonomously so do not ask for approval, simply proceed to implement." < /dev/null
    fi
    
    if [ "$START_PHASE" -le 3 ]; then
        save_state "$i" 3
        echo "--- Phase 3: E2E Testing and PR ---" | tee -a "$LOG_FILE"
        run_agy_with_retry agy --dangerously-skip-permissions --print-timeout 1h -c --print "Verify the implementation using e2e testing. Once verified, open a pull request for the issue and close the issue. Do not merge the PR." < /dev/null
    fi
    
    if [ "$START_PHASE" -le 4 ]; then
        save_state "$i" 4
        echo "--- Phase 4: Explain Diff ---" | tee -a "$LOG_FILE"
        run_agy_with_retry agy --dangerously-skip-permissions --print-timeout 1h -c --print "Use the explain-diff skill at /root/splash/.agents/skills/explain-diff/SKILL.md to explain the changes made in this iteration, and write the explanation as a comment on the PR." < /dev/null
    fi
    
    echo "Finished iteration $i." | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
    
    # Reset START_PHASE for the next iteration
    START_PHASE=1
    save_state "$((i + 1))" 1
done

echo "Successfully completed all $ITERATIONS iterations." | tee -a "$LOG_FILE"
# Clean up state file on success
rm -f "$STATE_FILE"
