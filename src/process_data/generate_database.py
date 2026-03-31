import os
import random
import string
from gensim import models
import numpy as np

def generate_text(base_text, target_size):
    words = base_text.split()
    if len(words) == 0:
        return base_text
    
    current_size = len(base_text.encode('utf-8'))
    if current_size >= target_size:
        return base_text[:target_size]
    
    augmented_text = base_text
    while len(augmented_text.encode('utf-8')) < target_size:
        chunk_size = random.randint(50, 200)
        chunk = " ".join(random.choices(words, k=chunk_size))
        augmented_text = augmented_text + " " + chunk
    
    if len(augmented_text.encode('utf-8')) > target_size:
        augmented_text = augmented_text[:target_size]
    
    return augmented_text

def main():
    database_folder = "database"
    num_seed_files = 16384
    total_files = 262144
    
    if not os.path.exists(database_folder):
        print(f"Error: Database folder '{database_folder}' does not exist!")
        return
    
    print(f"Checking for seed files in {database_folder}...")
    
    seed_documents = []
    missing_files = []
    
    for i in range(num_seed_files):
        filename = f"{i}.txt"
        filepath = os.path.join(database_folder, filename)
        
        if not os.path.exists(filepath):
            missing_files.append(filename)
            continue
            
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                lines = f.readlines()
                if len(lines) < 3:
                    print(f"Warning: {filename} has less than 3 lines, skipping...")
                    continue
                    
                title = lines[0].strip()
                metadata = lines[1].strip()
                content = "".join(lines[2:]).strip()
                
                if not content:
                    print(f"Warning: {filename} has empty content, skipping...")
                    continue
                    
                seed_documents.append({
                    'title': title,
                    'metadata': metadata,
                    'content': content,
                    'filename': filename
                })
        except Exception as e:
            print(f"Error reading {filename}: {e}")
            continue
    
    print(f"Successfully loaded {len(seed_documents)} seed documents")
    
    if missing_files:
        print(f"Missing {len(missing_files)} files: {missing_files[:10]}...")
    
    if len(seed_documents) == 0:
        print("Error: No valid seed documents found!")
        return
    
    print("Building text generation model...")
    all_texts = [doc['content'] for doc in seed_documents]
    texts_for_training = [text.split() for text in all_texts if len(text) > 100]
    
    word2vec_model = None
    if texts_for_training:
        try:
            word2vec_model = models.Word2Vec(texts_for_training, vector_size=100, window=5, min_count=1, workers=4)
            print("Word2Vec model built successfully")
        except Exception as e:
            print(f"Word2Vec model building failed: {e}")
    
    print("Generating new documents...")
    for i in range(num_seed_files, total_files):
        if i % 1000 == 0:
            print(f"Generated {i - num_seed_files} documents...")
        
        seed_idx = i % len(seed_documents)
        seed_doc = seed_documents[seed_idx]
        
        target_size = random.randint(8 * 1024, 32 * 1024)
        
        base_content = seed_doc['content']
        new_content = generate_text(base_content, target_size)
        
        new_metadata = seed_doc['metadata']
        if len(new_metadata.encode('utf-8')) > 250:
            new_metadata = new_metadata[:250]
        elif len(new_metadata.encode('utf-8')) < 250:
            needed = 250 - len(new_metadata.encode('utf-8'))
            new_metadata = new_metadata + ''.join(random.choices(string.ascii_letters + string.digits, k=needed))
        
        new_title = f"Document_{i}"
        
        output_filename = f"{i}.txt"
        output_filepath = os.path.join(database_folder, output_filename)
        
        try:
            with open(output_filepath, 'w', encoding='utf-8') as f:
                f.write(new_title + "\n")
                f.write(new_metadata + "\n")
                f.write(new_content)
        except Exception as e:
            print(f"Error writing {output_filename}: {e}")
            continue
        
        if i % 1000 == 0 and i > num_seed_files:
            new_seed = {
                'title': new_title,
                'metadata': new_metadata,
                'content': new_content,
                'filename': output_filename
            }
            if len(new_content.split()) > 50:
                seed_documents.append(new_seed)
                if len(seed_documents) > 2000:
                    seed_documents = seed_documents[-1500:]
    
    print("Document generation completed successfully!")

if __name__ == "__main__":
    main()
