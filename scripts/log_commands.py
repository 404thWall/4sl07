import json
import uuid
from datetime import datetime
from pathlib import Path

CONFIG_FILE = Path("./deploy_command.json")
HISTORY_FILE = Path("./deploy_history.jsonl")

def load_config():
    if CONFIG_FILE.exists():
        try:
            with open(CONFIG_FILE, "r") as f:
                return json.load(f)
        except (json.JSONDecodeError, IOError):
            return {}
    return {}

def save_config(args_dict):
    to_save = {
        k: (str(v) if isinstance(v, Path) else v) 
        for k, v in args_dict.items() 
        if k not in ['save', 'kill'] and v is not None
    }
    with open(CONFIG_FILE, "w") as f:
        json.dump(to_save, f, indent=4)
        print(f"Config updated in {CONFIG_FILE}")

def log_execution(args_dict: dict, status: str = "started", session_id: str = None) -> str:
    """
    If session_id is None, creates a new log entry and returns the new ID.
    If session_id is provided, updates the status of that specific entry.
    """
    now = datetime.now().isoformat()
    
    if session_id is None:
        session_id = str(uuid.uuid4())
        log_entry = {
            "status": status,
            "action": "kill_only" if args_dict.get("kill") else "deploy",
            "params": {
                k: (str(v) if isinstance(v, Path) else v)
                for k, v in args_dict.items()
                if k not in ['save']
            },
            "timestamp": now,
            "end_time": None,
            "id": session_id,
        }
        with open(HISTORY_FILE, "a") as f:
            f.write(json.dumps(log_entry) + "\n")
        return session_id
    else:
        # Update existing log
        lines = []
        if HISTORY_FILE.exists():
            with open(HISTORY_FILE, "r") as f:
                for line in f:
                    data = json.loads(line)
                    if data.get("id") == session_id:
                        data["status"] = status
                        data["end_time"] = now
                    lines.append(json.dumps(data))
            
            with open(HISTORY_FILE, "w") as f:
                f.write("\n".join(lines) + "\n")
        return session_id