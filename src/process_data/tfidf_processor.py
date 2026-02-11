import os
import re
import math
from collections import defaultdict, Counter
import numpy as np
from concurrent.futures import ProcessPoolExecutor
import multiprocessing as mp

class TFIDFProcessor:
    def __init__(self, db_folder, output_file, max_words=65536, num_bins=1024):
        self.db_folder = db_folder
        self.output_file = output_file
        self.max_words = max_words
        self.num_bins = num_bins
        
        self.txt_files = sorted(
            [f for f in os.listdir(db_folder) if f.endswith('.txt')],
            key=lambda x: int(x.split('.')[0])
        )
        self.num_docs = len(self.txt_files)
        
        self.word_to_id = {}
        self.id_to_word = {}
        self.doc_freq = defaultdict(int)
        self.doc_word_counts = []
        self.tfidf_matrix = None
        
    def clean_text(self, text):
        if not text:
            return []
        
        text = text.lower()
        
        text = re.sub(r'[^\w\s]', ' ', text)
        
        words = text.split()
        
        stop_words = {'a', 'an', 'the', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'by'}
        meaningful_words = []
        
        for word in words:
            if len(word) > 2 and word not in stop_words:
                if word.endswith(('ing', 'ed', 's', 'es')):
                    word = re.sub(r'(ing|ed|s|es)$', '', word)
                meaningful_words.append(word)
                
        return meaningful_words
    
    def process_document(self, file_idx, file_name):
        file_path = os.path.join(self.db_folder, file_name)
        
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            lines = f.readlines()
            
        if len(lines) >= 3:
            content = ' '.join(lines[2:])
            words = self.clean_text(content)
            word_counts = Counter(words)
            return file_idx, word_counts
        else:
            return file_idx, Counter()
    
    def build_vocabulary(self, num_workers=None):
        if num_workers is None:
            num_workers = mp.cpu_count()
        
        print(f"Processing {self.num_docs} documents using {num_workers} processes...")
        
        with ProcessPoolExecutor(max_workers=num_workers) as executor:
            futures = []
            for i, file_name in enumerate(self.txt_files):
                futures.append(executor.submit(self.process_document, i, file_name))
            
            word_doc_counts = defaultdict(int)
            self.doc_word_counts = [None] * self.num_docs
            
            for i, future in enumerate(futures):
                doc_idx, word_counts = future.result()
                self.doc_word_counts[doc_idx] = word_counts
                
                for word in set(word_counts.keys()):
                    word_doc_counts[word] += 1
                
                if (i + 1) % 1000 == 0:
                    print(f"Processed {i + 1}/{self.num_docs} documents")
        
        print(f"Vocabulary size: {len(word_doc_counts)} unique words")
        
        idf_scores = {}
        for word, doc_count in word_doc_counts.items():
            idf_scores[word] = math.log(self.num_docs / (1 + doc_count))
        
        top_words = sorted(idf_scores.items(), key=lambda x: x[1], reverse=True)[:self.max_words]
        
        for idx, (word, _) in enumerate(top_words):
            self.word_to_id[word] = idx
            self.id_to_word[idx] = word
        
        self.doc_freq = {word: word_doc_counts[word] for word in self.word_to_id.keys()}
        
        print(f"Selected {len(self.word_to_id)} words with highest IDF")
        
        return top_words
    
    def calculate_tfidf_matrix(self):
        num_docs = self.num_docs
        num_words = len(self.word_to_id)
        
        tfidf_matrix = np.zeros((num_docs, num_words), dtype=np.float32)
        
        print("Calculating TF-IDF matrix...")
        
        for doc_idx in range(num_docs):
            word_counts = self.doc_word_counts[doc_idx]
            total_words = sum(word_counts.values())
            
            if total_words == 0:
                continue
                
            for word, count in word_counts.items():
                if word in self.word_to_id:
                    word_id = self.word_to_id[word]
                    
                    tf = count / total_words
                    
                    idf = math.log(num_docs / (1 + self.doc_freq[word]))
                    
                    tfidf_matrix[doc_idx, word_id] = tf * idf
            
            if (doc_idx + 1) % 1000 == 0:
                print(f"Calculated {doc_idx + 1}/{num_docs} documents")
        
        self.tfidf_matrix = tfidf_matrix
        return tfidf_matrix
    
    def quantize_matrix(self, tfidf_matrix):
        print("Quantizing matrix...")
        
        global_min = tfidf_matrix.min()
        global_max = tfidf_matrix.max()
        
        print(f"TF-IDF value range: {global_min:.6f} - {global_max:.6f}")
        
        if global_max - global_min < 1e-10:
            quantized_matrix = np.zeros_like(tfidf_matrix, dtype=np.int16)
        else:
            normalized = (tfidf_matrix - global_min) / (global_max - global_min)
            quantized_matrix = (normalized * (self.num_bins - 1)).astype(np.int16)
        
        return quantized_matrix
    
    def save_results(self, top_words, quantized_matrix):
        print("Saving results to file...")
        
        with open(self.output_file, 'w', encoding='utf-8') as f:
            words = [word for word, _ in top_words]
            f.write(','.join(words) + '\n')
            
            for i in range(self.num_docs):
                row_values = quantized_matrix[i].tolist()
                row_str = ','.join(map(str, row_values))
                f.write(row_str + '\n')
        
        print(f"Results saved to {self.output_file}")
    
    def process(self):
        top_words = self.build_vocabulary()
        
        tfidf_matrix = self.calculate_tfidf_matrix()
        
        quantized_matrix = self.quantize_matrix(tfidf_matrix)
        
        self.save_results(top_words, quantized_matrix)
        
        print("Processing complete!")
        
        return top_words, quantized_matrix


def main():
    DB_FOLDER = "/home/lyl/Desktop/Qiyana/Qiyana_artifact/src/database"
    OUTPUT_FILE = "/home/lyl/Desktop/Qiyana/tf-idf.txt"
    MAX_WORDS = 65536
    NUM_BINS = 1024
    
    processor = TFIDFProcessor(DB_FOLDER, OUTPUT_FILE, MAX_WORDS, NUM_BINS)
    
    try:
        processor.process()
        print("\nTask completed!")
        print(f"Output file: {OUTPUT_FILE}")
        print(f"First line contains {MAX_WORDS} words")
        print(f"Following {processor.num_docs} lines each contain {MAX_WORDS} values (0-1023)")
    except Exception as e:
        print(f"Error during processing: {str(e)}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()