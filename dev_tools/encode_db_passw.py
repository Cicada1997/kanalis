def format_password(passw: str):
    result = ""
    for c in passw:
        if c.isalnum():
            result += c
        else:
            result += "%" + c.encode("utf-8").hex()

    return result

print(f"Your password in psql hex is: { format_password(input("Enter utf-8 encoded passowrd: ")) }")
