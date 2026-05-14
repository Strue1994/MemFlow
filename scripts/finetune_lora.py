"""Fine-tune a model using LoRA with Unsloth."""

import torch
from pathlib import Path


def check_dependencies():
    """Check if required dependencies are installed."""
    try:
        import unsloth
        return True
    except ImportError:
        print("Error: unsloth not installed")
        print("Run: pip install unsloth")
        return False


def prepare_model_config():
    """Return model configuration."""
    return {
        "model_name": "deepseek-ai/deepseek-coder-6.7b-instruct",
        "max_seq_length": 2048,
        "output_dir": "./lora-memflow",
        "rank": 16,
        "lora_alpha": 16,
    }


async def prepare_dataset(train_path: str, val_path: str):
    """Load and prepare dataset."""
    from datasets import load_dataset
    
    dataset = load_dataset("json", data_files={
        "train": train_path,
        "validation": val_path,
    })
    
    return dataset


def formatting_func(examples, tokenizer):
    """Format examples for training."""
    from tokenizer import Tokenizer
    
    texts = []
    for msgs in examples["messages"]:
        if isinstance(msgs, list):
            text = tokenizer.apply_chat_template(msgs, tokenize=False)
        else:
            text = str(msgs)
        texts.append(text)
    
    return {"text": texts}


def main():
    """Main fine-tuning entry point."""
    if not check_dependencies():
        return
    
    import argparse
    from unsloth import FastLanguageModel
    from transformers import TrainingArguments
    from trl import SFTTrainer
    
    parser = argparse.ArgumentParser(description="Fine-tune with LoRA")
    parser.add_argument("--model", default=None, help="Model name")
    parser.add_argument("--train", default="data/train.jsonl", help="Train data")
    parser.add_argument("--val", default="data/val.jsonl", help="Validation data")
    parser.add_argument("--output", default="./lora-memflow", help="Output directory")
    parser.add_argument("--epochs", type=int, default=3, help="Epochs")
    args = parser.parse_args()
    
    config = prepare_model_config()
    model_name = args.model or config["model_name"]
    max_seq_length = config["max_seq_length"]
    output_dir = args.output
    
    print(f"Loading model: {model_name}")
    
    # Load model and tokenizer
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=model_name,
        max_seq_length=max_seq_length,
        dtype=torch.float16,
        load_in_4bit=True,
    )
    
    # Configure LoRA
    model = FastLanguageModel.get_peft_model(
        model,
        r=config["rank"],
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
        lora_alpha=config["lora_alpha"],
        lora_dropout=0,
        bias="none",
        use_gradient_checkpointing=True,
        random_state=42,
    )
    
    print(f"Loading dataset: {args.train}")
    
    # Check if data exists
    train_path = Path(args.train)
    val_path = Path(args.val)
    
    if not train_path.exists():
        print(f"Training data not found: {args.train}")
        print("Run: python scripts/prepare_finetune_data.py first")
        return
    
    # Load dataset
    try:
        dataset = load_dataset("json", data_files={
            "train": str(train_path),
            "validation": str(val_path) if val_path.exists() else None,
        })
    except Exception as e:
        print(f"Error loading dataset: {e}")
        return
    
    # Create trainer
    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=dataset["train"],
        eval_dataset=dataset.get("validation"),
        formatting_func=lambda x: formatting_func(x, tokenizer),
        args=TrainingArguments(
            output_dir=output_dir,
            per_device_train_batch_size=2,
            gradient_accumulation_steps=4,
            num_train_epochs=args.epochs,
            logging_steps=10,
            save_steps=100,
            evaluation_strategy="steps" if val_path.exists() else "no",
            eval_steps=100 if val_path.exists() else None,
            save_total_limit=2,
            fp16=True,
            report_to="none",
        ),
    )
    
    print("Starting training...")
    trainer.train()
    
    # Save merged model
    print(f"Saving to {output_dir}")
    model.save_pretrained_merged(output_dir, tokenizer, save_method="merged_16bit")
    print("Done!")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())