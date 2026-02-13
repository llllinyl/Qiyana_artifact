def find_longest_keyword():
    with open("/root/Qiyana-experiment/keyword.txt", 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    max_length = 0
    longest_keyword = ""
    line_number = 0
    
    for i, line in enumerate(lines, 1):
        for keyword in line.strip().split(','):
            word = keyword.strip()
            if len(word) > max_length:
                max_length = len(word)
                longest_keyword = word
                line_number = i
    
    print(f"Longest keyword: \"{longest_keyword}\"")
    print(f"Length: {max_length} characters")
    print(f"Line number: {line_number}")

if __name__ == "__main__":
    find_longest_keyword()
