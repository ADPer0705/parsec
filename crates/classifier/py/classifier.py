"""
Classifier module using Hugging Face transformers for command/prompt classification.
"""

import json
import re
from typing import Dict, List, Optional, Tuple
from transformers import pipeline
import logging

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class CommandClassifier:
    def __init__(self, model_name: str = "facebook/bart-large-mnli"):
        """
        Initialize the classifier with a pre-trained model.
        
        Args:
            model_name: HuggingFace model name for zero-shot classification
        """
        self.model_name = model_name
        self.classifier = None
        self.shell_commands = {
            'ls', 'cd', 'pwd', 'mkdir', 'rm', 'cp', 'mv', 'cat', 'grep', 'find',
            'git', 'cargo', 'npm', 'python', 'node', 'curl', 'wget', 'ssh', 'scp',
            'vim', 'nano', 'emacs', 'docker', 'kubectl', 'make', 'sudo', 'chmod',
            'chown', 'ps', 'kill', 'top', 'htop', 'df', 'du', 'tar', 'unzip',
            'echo', 'touch', 'head', 'tail', 'sort', 'uniq', 'wc', 'awk', 'sed'
        }
        
        self.prompt_indicators = {
            'please', 'how do i', 'help me', 'can you', 'i need', 'i want',
            'what is', 'how to', 'show me', 'explain', 'create a', 'build a',
            'set up', 'configure', 'install', 'initialize', 'generate', 'make a'
        }
        
        self._initialize_model()
    
    def _initialize_model(self):
        """Initialize the Hugging Face model."""
        try:
            self.classifier = pipeline(
                "zero-shot-classification",
                model=self.model_name,
                device=-1  # Use CPU, set to 0 for GPU
            )
            logger.info(f"Initialized classifier with model: {self.model_name}")
        except Exception as e:
            logger.warning(f"Failed to initialize model {self.model_name}: {e}")
            logger.info("Falling back to heuristic classification")
    
    def _preprocess_input(self, text: str) -> str:
        """Preprocess input text for classification."""
        # Remove excessive whitespace
        text = ' '.join(text.strip().split())
        return text
    
    def _heuristic_classification(self, text: str) -> Tuple[str, float, str, Dict]:
        """
        Fallback heuristic classification when ML model is unavailable.
        
        Returns:
            Tuple of (classification, confidence, reasoning, metadata)
        """
        text_lower = text.lower().strip()
        first_word = text_lower.split()[0] if text_lower.split() else ""
        
        detected_patterns = []
        language_indicators = []
        
        # Check for shell command patterns
        if first_word in self.shell_commands:
            detected_patterns.append("command_verb")
            return (
                "shell",
                0.9,
                f"Detected shell command pattern with first word '{first_word}'",
                {
                    "detected_patterns": detected_patterns,
                    "language_indicators": language_indicators
                }
            )
        
        # Check for flag patterns
        if ' -' in text or ' --' in text:
            detected_patterns.append("flag_pattern")
        
        # Check for file paths
        if './' in text or '../' in text or '/' in first_word:
            detected_patterns.append("path_pattern")
        
        # Check for natural language indicators
        for indicator in self.prompt_indicators:
            if indicator in text_lower:
                language_indicators.append(indicator)
        
        # Check for question patterns
        if text.endswith('?') or any(text_lower.startswith(q) for q in ['what', 'how', 'why', 'when', 'where']):
            language_indicators.append("question_pattern")
        
        # Decision logic
        if detected_patterns and not language_indicators:
            return (
                "shell",
                0.8,
                f"Detected shell patterns: {detected_patterns}",
                {
                    "detected_patterns": detected_patterns,
                    "language_indicators": language_indicators
                }
            )
        elif language_indicators:
            return (
                "prompt",
                0.8,
                f"Detected natural language indicators: {language_indicators}",
                {
                    "detected_patterns": detected_patterns,
                    "language_indicators": language_indicators
                }
            )
        else:
            # Default to prompt for ambiguous cases
            return (
                "prompt",
                0.6,
                "Ambiguous input, defaulting to prompt classification",
                {
                    "detected_patterns": detected_patterns,
                    "language_indicators": language_indicators
                }
            )
    
    def _ml_classification(self, text: str) -> Tuple[str, float, str, Dict]:
        """
        ML-based classification using Hugging Face model.
        
        Returns:
            Tuple of (classification, confidence, reasoning, metadata)
        """
        candidate_labels = [
            "shell command execution",
            "natural language request",
            "system administration command",
            "conversational prompt"
        ]
        
        try:
            result = self.classifier(text, candidate_labels)
            
            # Get the best classification
            best_label = result['labels'][0]
            best_score = result['scores'][0]
            
            # Map labels to our classification system
            if best_label in ["shell command execution", "system administration command"]:
                classification = "shell"
            else:
                classification = "prompt"
            
            reasoning = f"ML model classified as '{best_label}' with confidence {best_score:.3f}"
            
            # Add heuristic patterns for additional metadata
            _, _, _, heuristic_metadata = self._heuristic_classification(text)
            
            return (
                classification,
                best_score,
                reasoning,
                {
                    "detected_patterns": heuristic_metadata["detected_patterns"],
                    "language_indicators": heuristic_metadata["language_indicators"],
                    "ml_label": best_label,
                    "ml_scores": dict(zip(result['labels'], result['scores']))
                }
            )
            
        except Exception as e:
            logger.warning(f"ML classification failed: {e}, falling back to heuristics")
            return self._heuristic_classification(text)
    
    def classify(self, text: str, context: Optional[Dict] = None) -> Dict:
        """
        Classify input text as shell command or natural language prompt.
        
        Args:
            text: Input text to classify
            context: Optional context with session_id and history
            
        Returns:
            Classification result as dictionary
        """
        if not text or not text.strip():
            return {
                "classification": "shell",
                "confidence": 1.0,
                "reasoning": "Empty input defaults to shell",
                "metadata": {
                    "detected_patterns": [],
                    "language_indicators": []
                }
            }
        
        processed_text = self._preprocess_input(text)
        
        # Use ML classification if model is available, otherwise use heuristics
        if self.classifier:
            classification, confidence, reasoning, metadata = self._ml_classification(processed_text)
        else:
            classification, confidence, reasoning, metadata = self._heuristic_classification(processed_text)
        
        # Adjust confidence based on context if available
        if context and context.get('history'):
            # This is a placeholder for context-aware classification
            # In a real implementation, you could use the command history
            # to improve classification accuracy
            pass
        
        return {
            "classification": classification,
            "confidence": confidence,
            "reasoning": reasoning,
            "metadata": metadata
        }

# Global classifier instance
_classifier = None

def get_classifier() -> CommandClassifier:
    """Get or create global classifier instance."""
    global _classifier
    if _classifier is None:
        _classifier = CommandClassifier()
    return _classifier

def classify_input(request_json: str) -> str:
    """
    Main entry point for classification called from Rust.
    
    Args:
        request_json: JSON string containing classification request
        
    Returns:
        JSON string containing classification response
    """
    try:
        request = json.loads(request_json)
        input_text = request['input']
        context = request.get('context')
        
        classifier = get_classifier()
        result = classifier.classify(input_text, context)
        
        return json.dumps(result)
        
    except Exception as e:
        logger.error(f"Classification error: {e}")
        # Return error response
        return json.dumps({
            "classification": "prompt",  # Safe default
            "confidence": 0.5,
            "reasoning": f"Classification error: {str(e)}",
            "metadata": {
                "detected_patterns": [],
                "language_indicators": [],
                "error": str(e)
            }
        })

if __name__ == "__main__":
    # Test the classifier
    test_inputs = [
        "ls -la",
        "git status",
        "Please help me create a new Rust project",
        "How do I initialize a git repository?",
        "cargo build --release",
        "Can you show me how to set up Docker?",
        "./configure --prefix=/usr/local",
        "What is the best way to handle errors in Rust?"
    ]
    
    classifier = CommandClassifier()
    
    for input_text in test_inputs:
        result = classifier.classify(input_text)
        print(f"Input: '{input_text}'")
        print(f"Classification: {result['classification']} (confidence: {result['confidence']:.3f})")
        print(f"Reasoning: {result['reasoning']}")
        print(f"Metadata: {result['metadata']}")
        print("-" * 50)
