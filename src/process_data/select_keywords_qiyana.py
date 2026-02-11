import os
import re
import math
from collections import defaultdict

def load_documents(database_folder):
    documents = []
    
    for i in range(16384):
        file_path = os.path.join(database_folder, f"{i}.txt")
        
        if os.path.exists(file_path):
            with open(file_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
                if len(lines) >= 3:
                    content = ''.join(lines[2:]).strip()
                    documents.append(content)
                else:
                    documents.append("")
        else:
            documents.append("")
            
    return documents

def preprocess_text_english(text):
    if not text:
        return []
    
    text = text.lower()
    
    words = re.findall(r'\b[a-zA-Z]{2,}\b', text)
    
    stopwords = set(['the', 'and', 'of', 'to', 'a', 'in', 'is', 'it',
                    'you', 'that', 'he', 'was', 'for', 'on', 'are',
                    'as', 'with', 'his', 'they', 'at', 'be', 'this',
                    'have', 'from', 'or', 'one', 'had', 'by', 'word',
                    'but', 'not', 'what', 'all', 'were', 'we', 'when',
                    'your', 'can', 'said', 'there', 'use', 'an', 'each',
                    'which', 'she', 'do', 'how', 'their', 'if', 'will',
                    'up', 'other', 'about', 'out', 'many', 'then', 'them',
                    'these', 'so', 'some', 'her', 'would', 'make', 'like',
                    'him', 'into', 'time', 'has', 'look', 'two', 'more',
                    'write', 'go', 'see', 'number', 'no', 'way', 'could',
                    'people', 'my', 'than', 'first', 'water', 'been', 'call',
                    'who', 'oil', 'its', 'now', 'find', 'long', 'down', 'day',
                    'did', 'get', 'come', 'made', 'may', 'part'])
    
    words = [word for word in words if word not in stopwords]
    
    return words

def calculate_tf_idf_english(documents):
    processed_docs = [preprocess_text_english(doc) for doc in documents]
    
    doc_freq = defaultdict(int)
    total_docs = len(processed_docs)
    
    for words in processed_docs:
        unique_words = set(words)
        for word in unique_words:
            doc_freq[word] += 1
    
    tfidf_results = []
    
    for words in processed_docs:
        if not words:
            tfidf_results.append([])
            continue
            
        word_count = len(words)
        tf = defaultdict(float)
        for word in words:
            tf[word] += 1
        
        word_tfidf = {}
        for word, freq in tf.items():
            tf_value = freq / word_count
            idf_value = math.log(total_docs / (doc_freq.get(word, 0) + 1))
            word_tfidf[word] = tf_value * idf_value
        
        sorted_words = sorted(word_tfidf.items(), key=lambda x: x[1], reverse=True)
        tfidf_results.append(sorted_words)
    
    return tfidf_results

def main_simple():
    database_folder = "/home/lyl/Desktop/Qiyana/Qiyana_artifact/src/database"
    output_file = "/home/lyl/Desktop/Qiyana/keyword.txt"
    num_keywords = 8
    
    print("Loading documents...")
    documents = load_documents(database_folder)
    
    print("Calculating TF-IDF...")
    tfidf_results = calculate_tf_idf_english(documents)
    
    print("Extracting keywords...")
    with open(output_file, 'w', encoding='utf-8') as f:
        for sorted_words in tfidf_results:
            if sorted_words:
                keywords = [word for word, score in sorted_words[:num_keywords]]
                f.write(",".join(keywords) + "\n")
            else:
                f.write("\n")
    
    print(f"Keywords saved to {output_file}")

if __name__ == "__main__":
    main_simple()
