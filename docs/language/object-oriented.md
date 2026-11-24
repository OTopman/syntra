# Object-Oriented Programming

Syntra provides a clean, class-based object-oriented model with encapsulation, constructors, and interface contracts.

---

## 🏛️ Class Declarations

A class declares fields, a constructor (`new`), and methods:

```syntra
class BankAccount {
    public int accountNumber;
    public string owner;
    private float balance;

    // Constructor
    new(int accountNumber, string owner, float initialDeposit) {
        this.accountNumber = accountNumber;
        this.owner = owner;
        this.balance = initialDeposit;
    }

    public func deposit(float amount): void {
        if amount > 0.0 {
            this.balance = this.balance + amount;
        }
    }

    public func withdraw(float amount): bool {
        if amount > 0.0 && this.balance >= amount {
            this.balance = this.balance - amount;
            return true;
        }
        return false;
    }

    public func get_balance(): float {
        return this.balance;
    }
}
```

### Constructor & Instantiation
Instantiate classes with the `new` operator:

```syntra
let account = new BankAccount(1001, "Alice", 500.0);
account.deposit(250.0);
System.println("Balance: " + account.get_balance());
```

---

## 🔒 Visibility Modifiers

* **`public`**: Field or method is accessible from any scope.
* **`private`**: Field or method is accessible only from within the declaring class.
* If omitted, class members default to `private` for fields and `public` for methods.

---

## 📜 Interfaces (`interface` & `implements`)

Interfaces define structural behavior contracts that implementing classes must satisfy:

```syntra
interface Serializable {
    func serialize(): string;
}

class UserProfile implements Serializable {
    public string username;
    public string email;

    new(string username, string email) {
        this.username = username;
        this.email = email;
    }

    public func serialize(): string {
        return "{\"username\":\"" + this.username + "\",\"email\":\"" + this.email + "\"}";
    }
}
```
