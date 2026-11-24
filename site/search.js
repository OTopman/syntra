// Syntra Documentation Engine: Instant Fuzzy Search & Theme Toggle
const SEARCH_INDEX = [
  {"name":"air","kind":"module","module":"air","parent":null,"url":"air.html","desc":"Conforms to Specification 03: AIR-001 - AIR-032. Implements SSA Intermediate Representation generation, block branching,"},
  {"name":"AirValue","kind":"class","module":"air","parent":null,"url":"air.html#class-AirValue","desc":"SSA Value Identifier (e.g. `v0`, `v1`, `v2`).  Every value in AIR represents an immutable single-assignment static singl"},
  {"name":"toString","kind":"method","module":"air","parent":"AirValue","url":"air.html#method-AirValue-toString","desc":"Formats the register as `\"v<id>\"`."},
  {"name":"AirInstruction","kind":"class","module":"air","parent":null,"url":"air.html#class-AirInstruction","desc":"Low-level Static Single Assignment (SSA) instruction in AIR.  Encapsulates opcode, target register, operands, and option"},
  {"name":"toString","kind":"method","module":"air","parent":"AirInstruction","url":"air.html#method-AirInstruction-toString","desc":"Serializes instruction into standard AIR text format: `v0: int = add v1, v2`."},
  {"name":"AirTerminator","kind":"class","module":"air","parent":null,"url":"air.html#class-AirTerminator","desc":"Basic block control-flow terminator instruction.  Represents function return (`ret`), unconditional branch (`br`), or co"},
  {"name":"toString","kind":"method","module":"air","parent":"AirTerminator","url":"air.html#method-AirTerminator-toString","desc":"Formats the terminator instruction as an indented AIR line."},
  {"name":"AirBlock","kind":"class","module":"air","parent":null,"url":"air.html#class-AirBlock","desc":"Basic Block in the SSA control-flow graph (CFG).  Consists of a sequence of straight-line instructions followed by exact"},
  {"name":"addInstruction","kind":"method","module":"air","parent":"AirBlock","url":"air.html#method-AirBlock-addInstruction","desc":"Appends an instruction to this block."},
  {"name":"setTerminator","kind":"method","module":"air","parent":"AirBlock","url":"air.html#method-AirBlock-setTerminator","desc":"Sets the block terminator instruction."},
  {"name":"toString","kind":"method","module":"air","parent":"AirBlock","url":"air.html#method-AirBlock-toString","desc":"Formats the block and all child instructions as textual AIR."},
  {"name":"AirFunction","kind":"class","module":"air","parent":null,"url":"air.html#class-AirFunction","desc":"Lowered AIR SSA Function representation.  Holds parameter names, types, return type signature, and the collection of con"},
  {"name":"addParam","kind":"method","module":"air","parent":"AirFunction","url":"air.html#method-AirFunction-addParam","desc":"Adds a typed parameter to the function signature."},
  {"name":"addBlock","kind":"method","module":"air","parent":"AirFunction","url":"air.html#method-AirFunction-addBlock","desc":"Adds a basic block to the function CFG."},
  {"name":"toString","kind":"method","module":"air","parent":"AirFunction","url":"air.html#method-AirFunction-toString","desc":"Serializes the entire function signature and CFG blocks into text AIR format."},
  {"name":"AirModule","kind":"class","module":"air","parent":null,"url":"air.html#class-AirModule","desc":"Top-level compilation module containing SSA functions.  Encapsulates all lowered functions for a compilation unit."},
  {"name":"addFunction","kind":"method","module":"air","parent":"AirModule","url":"air.html#method-AirModule-addFunction","desc":"Appends a lowered AIR function to the module."},
  {"name":"toString","kind":"method","module":"air","parent":"AirModule","url":"air.html#method-AirModule-toString","desc":"Serializes the entire module into textual AIR syntax."},
  {"name":"SsaVar","kind":"class","module":"air","parent":null,"url":"air.html#class-SsaVar","desc":"Variable-to-SSA register mapping binding.  Used during lowering to track which `AirValue` currently holds a lexical vari"},
  {"name":"AirLowering","kind":"class","module":"air","parent":null,"url":"air.html#class-AirLowering","desc":"AIR Lowering Engine: converts type-checked AST nodes to SSA AIR modules.  Performs control-flow graph construction, basi"},
  {"name":"newValue","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-newValue","desc":"Allocates a new virtual SSA value register of the specified type."},
  {"name":"newBlock","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-newBlock","desc":"Allocates a new basic block with a unique sequential ID."},
  {"name":"bindVar","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-bindVar","desc":"Updates or inserts a variable binding to point to an SSA value register."},
  {"name":"lookupVar","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lookupVar","desc":"Looks up the current SSA value register bound to a lexical variable name."},
  {"name":"emit","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-emit","desc":"Emits a new SSA instruction into the active basic block and returns its result register."},
  {"name":"lowerProgram","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lowerProgram","desc":"Lowers an entire parsed and verified program AST into an SSA `AirModule`.  Iterates top-level functi"},
  {"name":"lowerFunction","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lowerFunction","desc":"Lowers a function declaration AST node into an `AirFunction`.  Creates entry block `bb0`, binds inco"},
  {"name":"lowerClass","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lowerClass","desc":"Lowers methods of a class declaration into namespaced AIR functions with explicit `this` pointer par"},
  {"name":"lowerBlock","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lowerBlock","desc":"Lowers an AST statement block node into the active CFG block stream."},
  {"name":"lowerStatement","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lowerStatement","desc":"Dispatches lowering of an individual AST statement (variable declarations, returns, control flow)."},
  {"name":"lowerIfStmt","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lowerIfStmt","desc":"Lowers an `if-else` branching statement, splitting control flow into `then`, `else`, and `merge` blo"},
  {"name":"lowerWhileStmt","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lowerWhileStmt","desc":"Lowers a `while` loop statement into `cond`, `body`, and `exit` blocks with backward loop branch."},
  {"name":"lowerExpression","kind":"method","module":"air","parent":"AirLowering","url":"air.html#method-AirLowering-lowerExpression","desc":"Lowers an AST expression into one or more linearized SSA instructions and returns the result value. "},
  {"name":"checker","kind":"module","module":"checker","parent":null,"url":"checker.html","desc":"Provides a zero-GC self-hosting Type Checker, Symbol Table, and Scope Analyzer in Syntra."},
  {"name":"Type","kind":"class","module":"checker","parent":null,"url":"checker.html#class-Type","desc":"Type representation in the Syntra semantic model.  Encapsulates primitive types (`int`, `float`, `bool`, `string`, `void"},
  {"name":"isNumeric","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-isNumeric","desc":"Checks if this type represents a scalar numeric type (`int` or `float`)."},
  {"name":"isBool","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-isBool","desc":"Checks if this type represents a scalar boolean type."},
  {"name":"isString","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-isString","desc":"Checks if this type represents a string."},
  {"name":"isVoid","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-isVoid","desc":"Checks if this type represents the `void` return type."},
  {"name":"isAssignableTo","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-isAssignableTo","desc":"Determines whether this type is assignable to the target type under Syntra typing rules.  Allows imp"},
  {"name":"toString","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-toString","desc":"Formats the type into canonical Syntra type syntax (e.g. `\"int\"`, `\"string[]\"`, `\"Context\"`)."},
  {"name":"Int","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-Int","desc":"Constructs the scalar integer type (`int`)."},
  {"name":"Float","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-Float","desc":"Constructs the scalar floating-point type (`float`)."},
  {"name":"Bool","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-Bool","desc":"Constructs the scalar boolean type (`bool`)."},
  {"name":"String","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-String","desc":"Constructs the UTF-8 string type (`string`)."},
  {"name":"Void","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-Void","desc":"Constructs the unit/void type (`void`)."},
  {"name":"Unknown","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-Unknown","desc":"Constructs an un-inferred placeholder type (`unknown`)."},
  {"name":"Class","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-Class","desc":"Constructs a nominal user-defined class type reference."},
  {"name":"Array","kind":"method","module":"checker","parent":"Type","url":"checker.html#method-Type-Array","desc":"Constructs a 1-dimensional array type of the given element type."},
  {"name":"Diagnostic","kind":"class","module":"checker","parent":null,"url":"checker.html#class-Diagnostic","desc":"Structured semantic diagnostic emitted during type checking.  Contains error code (e.g. `\"LANG-020\"`), descriptive error"},
  {"name":"toString","kind":"method","module":"checker","parent":"Diagnostic","url":"checker.html#method-Diagnostic-toString","desc":"Formats the diagnostic into a compiler error message line."},
  {"name":"VarSymbol","kind":"class","module":"checker","parent":null,"url":"checker.html#class-VarSymbol","desc":"Variable symbol in lexical scope.  Tracks identifier name, semantic type, mutability (`isConst`), and declaration span."},
  {"name":"FnSymbol","kind":"class","module":"checker","parent":null,"url":"checker.html#class-FnSymbol","desc":"Function symbol record with parameter signatures and return type.  Tracks function name, formal parameter names, paramet"},
  {"name":"addParam","kind":"method","module":"checker","parent":"FnSymbol","url":"checker.html#method-FnSymbol-addParam","desc":"Registers a formal parameter on this function."},
  {"name":"paramCount","kind":"method","module":"checker","parent":"FnSymbol","url":"checker.html#method-FnSymbol-paramCount","desc":"Returns the total formal parameter count."},
  {"name":"ClassSymbol","kind":"class","module":"checker","parent":null,"url":"checker.html#class-ClassSymbol","desc":"Class symbol table entry tracking fields and methods.  Encapsulates class name, field names, field types, and member met"},
  {"name":"addField","kind":"method","module":"checker","parent":"ClassSymbol","url":"checker.html#method-ClassSymbol-addField","desc":"Adds a typed field member to the class."},
  {"name":"addMethod","kind":"method","module":"checker","parent":"ClassSymbol","url":"checker.html#method-ClassSymbol-addMethod","desc":"Adds a member method symbol to the class."},
  {"name":"getFieldType","kind":"method","module":"checker","parent":"ClassSymbol","url":"checker.html#method-ClassSymbol-getFieldType","desc":"Retrieves the type of a declared field by name."},
  {"name":"hasField","kind":"method","module":"checker","parent":"ClassSymbol","url":"checker.html#method-ClassSymbol-hasField","desc":"Checks if a field exists on this class."},
  {"name":"getMethod","kind":"method","module":"checker","parent":"ClassSymbol","url":"checker.html#method-ClassSymbol-getMethod","desc":"Retrieves a member method symbol by name."},
  {"name":"hasMethod","kind":"method","module":"checker","parent":"ClassSymbol","url":"checker.html#method-ClassSymbol-hasMethod","desc":"Checks if a method exists on this class."},
  {"name":"Scope","kind":"class","module":"checker","parent":null,"url":"checker.html#class-Scope","desc":"Nested lexical scope frame for variable resolution and shadowing.  Implements standard innermost-first scope resolution."},
  {"name":"define","kind":"method","module":"checker","parent":"Scope","url":"checker.html#method-Scope-define","desc":"Declares a variable symbol in this scope frame."},
  {"name":"lookup","kind":"method","module":"checker","parent":"Scope","url":"checker.html#method-Scope-lookup","desc":"Looks up a variable by name within this scope frame (innermost-first)."},
  {"name":"contains","kind":"method","module":"checker","parent":"Scope","url":"checker.html#method-Scope-contains","desc":"Checks if a variable name is defined in this scope frame."},
  {"name":"TypeChecker","kind":"class","module":"checker","parent":null,"url":"checker.html#class-TypeChecker","desc":"Syntra Self-Hosting Type Checker and Semantic Analyzer (Specification 01: LANG-020).  Executes two-pass semantic analysi"},
  {"name":"registerBuiltins","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-registerBuiltins","desc":"Registers intrinsic compiler built-in symbols into the symbol table.  Registers: - `print(msg)` - `p"},
  {"name":"enterScope","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-enterScope","desc":"Pushes a new lexical scope frame onto the scope stack."},
  {"name":"exitScope","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-exitScope","desc":"Pops the active lexical scope frame off the scope stack."},
  {"name":"defineVar","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-defineVar","desc":"Defines a variable symbol in the topmost lexical scope frame."},
  {"name":"lookupVar","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-lookupVar","desc":"Resolves a variable symbol by searching from topmost to outermost scope frame."},
  {"name":"hasVar","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-hasVar","desc":"Checks whether a variable is visible in any active lexical scope frame."},
  {"name":"getFunction","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-getFunction","desc":"Looks up a registered function symbol by name."},
  {"name":"hasFunction","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-hasFunction","desc":"Checks if a global function is declared."},
  {"name":"getClass","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-getClass","desc":"Looks up a registered class symbol by name."},
  {"name":"hasClass","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-hasClass","desc":"Checks if a class is declared."},
  {"name":"error","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-error","desc":"Emits a diagnostic error record with standard error code `\"LANG-020\"`."},
  {"name":"errorCount","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-errorCount","desc":"Returns the total number of diagnostic errors recorded."},
  {"name":"hasErrors","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-hasErrors","desc":"Checks if any diagnostic errors were encountered during checking."},
  {"name":"parseType","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-parseType","desc":"Parses a textual type string into a semantic `Type` instance.  Handles array brackets `[]`, scalar t"},
  {"name":"collectSymbols","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-collectSymbols","desc":"Scans the root program AST and populates symbol tables for functions, classes, and namespaces."},
  {"name":"checkProgram","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkProgram","desc":"Executes complete semantic validation over an AST program tree.  Runs symbol collection followed by "},
  {"name":"checkFunctionDecl","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkFunctionDecl","desc":"Checks a function declaration, binding its formal parameters into scope and verifying its body."},
  {"name":"checkClassDecl","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkClassDecl","desc":"Checks a class declaration, verifying fields, constructors, and methods with bound `this` references"},
  {"name":"checkBlock","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkBlock","desc":"Checks a block statement in an isolated child scope."},
  {"name":"checkStatement","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkStatement","desc":"Dispatches checking of an individual AST statement."},
  {"name":"checkVarDecl","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkVarDecl","desc":"Checks a variable declaration, ensuring initial value is assignable to declared type."},
  {"name":"checkIfStmt","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkIfStmt","desc":"Checks an `if` conditional statement, asserting condition expression evaluates to `bool`."},
  {"name":"checkWhileStmt","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkWhileStmt","desc":"Checks a `while` loop statement, asserting loop condition evaluates to `bool`."},
  {"name":"checkReturnStmt","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkReturnStmt","desc":"Checks a `return` statement, validating returned expression against enclosing function return type."},
  {"name":"checkExpression","kind":"method","module":"checker","parent":"TypeChecker","url":"checker.html#method-TypeChecker-checkExpression","desc":"Performs type inference and validation on an arbitrary AST expression node.  Evaluates literals, ide"},
  {"name":"codegen","kind":"module","module":"codegen","parent":null,"url":"codegen.html","desc":"Translates Syntra AIR (SSA Intermediate Representation) into native machine assembly (x86-64 / AMD64 System V ABI). Full"},
  {"name":"AsmInstruction","kind":"class","module":"codegen","parent":null,"url":"codegen.html#class-AsmInstruction","desc":"Native Assembly Instruction representation.  Encapsulates mnemonic, destination operand, and source operand formatted fo"},
  {"name":"toString","kind":"method","module":"codegen","parent":"AsmInstruction","url":"codegen.html#method-AsmInstruction-toString","desc":"Formats the instruction as an indented assembly code line."},
  {"name":"AsmBlock","kind":"class","module":"codegen","parent":null,"url":"codegen.html#class-AsmBlock","desc":"Native Assembly Basic Block with label and sequential instruction stream.  Represents an un-branched linear sequence of "},
  {"name":"emit","kind":"method","module":"codegen","parent":"AsmBlock","url":"codegen.html#method-AsmBlock-emit","desc":"Emits and appends a new instruction into this block."},
  {"name":"toString","kind":"method","module":"codegen","parent":"AsmBlock","url":"codegen.html#method-AsmBlock-toString","desc":"Serializes the block with its header label and all constituent instructions."},
  {"name":"AsmFunction","kind":"class","module":"codegen","parent":null,"url":"codegen.html#class-AsmFunction","desc":"Native Assembly Function representation.  Manages stack frame size allocation, calling convention prologue/epilogue emis"},
  {"name":"addBlock","kind":"method","module":"codegen","parent":"AsmFunction","url":"codegen.html#method-AsmFunction-addBlock","desc":"Appends a basic block into the function."},
  {"name":"toString","kind":"method","module":"codegen","parent":"AsmFunction","url":"codegen.html#method-AsmFunction-toString","desc":"Emits the function definition including Mach-O / ELF export symbol, prologue, body blocks, and epilo"},
  {"name":"NativeCodegen","kind":"class","module":"codegen","parent":null,"url":"codegen.html#class-NativeCodegen","desc":"Self-Hosting Native Machine Code Generator.  Translates SSA AIR basic blocks and instructions into x86-64 / AMD64 System"},
  {"name":"getSlot","kind":"method","module":"codegen","parent":"NativeCodegen","url":"codegen.html#method-NativeCodegen-getSlot","desc":"Computes the stack memory slot displacement string for a virtual SSA value ID."},
  {"name":"generateModule","kind":"method","module":"codegen","parent":"NativeCodegen","url":"codegen.html#method-NativeCodegen-generateModule","desc":"Compiles all functions in the AIR module into a standalone assembly source text."},
  {"name":"generateFunction","kind":"method","module":"codegen","parent":"NativeCodegen","url":"codegen.html#method-NativeCodegen-generateFunction","desc":"Lowers a single AIR function into its native assembly function representation.  Computes stack frame"},
  {"name":"parseArgList","kind":"method","module":"codegen","parent":"NativeCodegen","url":"codegen.html#method-NativeCodegen-parseArgList","desc":"Parses a comma-separated list of value references into an integer array of value IDs."},
  {"name":"parseValueId","kind":"method","module":"codegen","parent":"NativeCodegen","url":"codegen.html#method-NativeCodegen-parseValueId","desc":"Parses an SSA value token (e.g. `\"v4\"`) into its integer ID (`4`)."},
  {"name":"lowerInstruction","kind":"method","module":"codegen","parent":"NativeCodegen","url":"codegen.html#method-NativeCodegen-lowerInstruction","desc":"Lowers an individual SSA AIR instruction into one or more native x86-64 instructions.  Handles const"},
  {"name":"lowerTerminator","kind":"method","module":"codegen","parent":"NativeCodegen","url":"codegen.html#method-NativeCodegen-lowerTerminator","desc":"Lowers an SSA block terminator into control-flow branches, jumps, or function epilogue return."},
  {"name":"concurrency","kind":"module","module":"concurrency","parent":null,"url":"concurrency.html","desc":"Standard Library: Concurrency & Thread Synchronization Provides Task, Channel, ThreadPool, and Select primitives"},
  {"name":"sleepMs","kind":"function","module":"concurrency","parent":null,"url":"concurrency.html#func-sleepMs","desc":"Suspends execution of the current fiber or thread for the specified duration.  Yields CPU execution "},
  {"name":"createChannel","kind":"function","module":"concurrency","parent":null,"url":"concurrency.html#func-createChannel","desc":"Creates a new unbounded FIFO synchronization channel for message passing.  Channels provide thread-s"},
  {"name":"createThreadPool","kind":"function","module":"concurrency","parent":null,"url":"concurrency.html#func-createThreadPool","desc":"Creates a bounded worker thread pool for parallel CPU-bound compute tasks.  Worker threads are poole"},
  {"name":"selectChannels","kind":"function","module":"concurrency","parent":null,"url":"concurrency.html#func-selectChannels","desc":"Non-blocking multiplexer over an array of channels.  Inspects the provided channels and returns the "},
  {"name":"crypto","kind":"module","module":"crypto","parent":null,"url":"crypto.html","desc":"100% Pure Syntra Implementation with ZERO external C or OpenSSL dependencies. Standards compliant: - SHA-256: FIPS PUB 1"},
  {"name":"Hex","kind":"class","module":"crypto","parent":null,"url":"crypto.html#class-Hex","desc":"High-performance hexadecimal serialization and deserialization utilities.  Encodes raw binary byte arrays into lowercase"},
  {"name":"encode","kind":"method","module":"crypto","parent":"Hex","url":"crypto.html#method-Hex-encode","desc":"Encodes an array of byte values (0-255) into a lowercase hexadecimal string."},
  {"name":"decode","kind":"method","module":"crypto","parent":"Hex","url":"crypto.html#method-Hex-decode","desc":"Decodes a hexadecimal string into an array of integer byte values.  Handles both uppercase (`A-F`) a"},
  {"name":"hexDigit","kind":"method","module":"crypto","parent":"Hex","url":"crypto.html#method-Hex-hexDigit","desc":"Converts an ASCII character code into its corresponding numeric nibble (0-15)."},
  {"name":"Base64","kind":"class","module":"crypto","parent":null,"url":"crypto.html#class-Base64","desc":"RFC 4648 Base64 serialization and deserialization subsystem.  Converts arbitrary binary buffers into URL-safe or standar"},
  {"name":"encode","kind":"method","module":"crypto","parent":"Base64","url":"crypto.html#method-Base64-encode","desc":"Encodes an array of binary bytes into standard RFC 4648 Base64 format with `=` padding."},
  {"name":"encodeString","kind":"method","module":"crypto","parent":"Base64","url":"crypto.html#method-Base64-encodeString","desc":"Encodes an input UTF-8 string into RFC 4648 Base64."},
  {"name":"decode","kind":"method","module":"crypto","parent":"Base64","url":"crypto.html#method-Base64-decode","desc":"Decodes an RFC 4648 Base64 string into an array of integer byte values."},
  {"name":"decodeString","kind":"method","module":"crypto","parent":"Base64","url":"crypto.html#method-Base64-decodeString","desc":"Decodes an RFC 4648 Base64 string directly into a UTF-8 string."},
  {"name":"buildReverseTable","kind":"method","module":"crypto","parent":"Base64","url":"crypto.html#method-Base64-buildReverseTable","desc":"Builds a 256-element reverse lookup table for fast Base64 decoding."},
  {"name":"Sha256","kind":"class","module":"crypto","parent":null,"url":"crypto.html#class-Sha256","desc":"NIST FIPS PUB 180-4 Secure Hash Standard (SHA-256) implementation.  Implements 256-bit cryptographic digest hashing with"},
  {"name":"getK","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-getK","desc":"Returns the 64 round constants from the fractional cube roots of the first 64 prime numbers."},
  {"name":"rotr","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-rotr","desc":"Performs bitwise 32-bit right rotation: `ROTR(x, n)`."},
  {"name":"ch","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-ch","desc":"SHA-256 bitwise choice function: `Ch(x, y, z) = (x & y) ^ (~x & z)`."},
  {"name":"maj","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-maj","desc":"SHA-256 majority bitwise function: `Maj(x, y, z) = (x & y) ^ (x & z) ^ (y & z)`."},
  {"name":"sigma0","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-sigma0","desc":"SHA-256 uppercase sigma-0 round function: `ROTR^2(x) ^ ROTR^13(x) ^ ROTR^22(x)`."},
  {"name":"sigma1","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-sigma1","desc":"SHA-256 uppercase sigma-1 round function: `ROTR^6(x) ^ ROTR^11(x) ^ ROTR^25(x)`."},
  {"name":"gamma0","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-gamma0","desc":"SHA-256 lowercase gamma-0 message schedule function: `ROTR^7(x) ^ ROTR^18(x) ^ (x >> 3)`."},
  {"name":"gamma1","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-gamma1","desc":"SHA-256 lowercase gamma-1 message schedule function: `ROTR^17(x) ^ ROTR^19(x) ^ (x >> 10)`."},
  {"name":"digest","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-digest","desc":"Computes the raw 32-byte SHA-256 cryptographic hash of an input byte array."},
  {"name":"hash","kind":"method","module":"crypto","parent":"Sha256","url":"crypto.html#method-Sha256-hash","desc":"Computes the 64-character lowercase hexadecimal SHA-256 digest of a string."},
  {"name":"HmacSha256","kind":"class","module":"crypto","parent":null,"url":"crypto.html#class-HmacSha256","desc":"RFC 2104 HMAC-SHA256 Keyed-Hashing for Message Authentication.  Produces cryptographically validated message authenticat"},
  {"name":"computeBytes","kind":"method","module":"crypto","parent":"HmacSha256","url":"crypto.html#method-HmacSha256-computeBytes","desc":"Computes HMAC-SHA256 over raw binary key and message byte arrays.  Handles key padding and reduction"},
  {"name":"compute","kind":"method","module":"crypto","parent":"HmacSha256","url":"crypto.html#method-HmacSha256-compute","desc":"Computes the 64-character lowercase hex HMAC-SHA256 signature for a string key and message."},
  {"name":"verify","kind":"method","module":"crypto","parent":"HmacSha256","url":"crypto.html#method-HmacSha256-verify","desc":"Verifies an HMAC-SHA256 signature using constant-time comparison immune to timing attacks."},
  {"name":"SecureRandom","kind":"class","module":"crypto","parent":null,"url":"crypto.html#class-SecureRandom","desc":"Cryptographically Secure Pseudorandom Number Generator (CSPRNG).  Implements Hash_DRBG-style entropy pooling based on na"},
  {"name":"bytes","kind":"method","module":"crypto","parent":"SecureRandom","url":"crypto.html#method-SecureRandom-bytes","desc":"Generates an array of cryptographically strong pseudo-random bytes."},
  {"name":"hex","kind":"method","module":"crypto","parent":"SecureRandom","url":"crypto.html#method-SecureRandom-hex","desc":"Generates a random lowercase hexadecimal string of the requested byte length."},
  {"name":"ConstantTime","kind":"class","module":"crypto","parent":null,"url":"crypto.html#class-ConstantTime","desc":"Constant-Time Comparison and Selection Utilities for Timing-Attack Mitigation.  Prevents side-channel timing attacks by "},
  {"name":"equals","kind":"method","module":"crypto","parent":"ConstantTime","url":"crypto.html#method-ConstantTime-equals","desc":"Compares two byte arrays in constant time."},
  {"name":"equalsHex","kind":"method","module":"crypto","parent":"ConstantTime","url":"crypto.html#method-ConstantTime-equalsHex","desc":"Compares two strings in constant time."},
  {"name":"select","kind":"method","module":"crypto","parent":"ConstantTime","url":"crypto.html#method-ConstantTime-select","desc":"Branchless conditional multiplexer selecting `x` when `cond != 0`, otherwise `y`.  Executes without "},
  {"name":"Zeroize","kind":"class","module":"crypto","parent":null,"url":"crypto.html#class-Zeroize","desc":"Sensitive Memory Buffer Overwrite Subsystem.  Securely overwrites cryptographic keys, nonces, and passwords with zeros i"},
  {"name":"wipe","kind":"method","module":"crypto","parent":"Zeroize","url":"crypto.html#method-Zeroize-wipe","desc":"Overwrites all elements of an integer byte buffer with zero values."},
  {"name":"email","kind":"module","module":"email","parent":null,"url":"email.html","desc":"Enterprise-grade, RFC-compliant email composition, MIME serialization, inbound parsing, DKIM canonicalization, SMTP clie"},
  {"name":"EmailAddress","kind":"class","module":"email","parent":null,"url":"email.html#class-EmailAddress","desc":"Represents a validated, structured RFC 5322 email address (display name and addr-spec).  Handles formatting and parsing "},
  {"name":"of","kind":"method","module":"email","parent":"EmailAddress","url":"email.html#method-EmailAddress-of","desc":"Constructs an `EmailAddress` with an address and optional display name."},
  {"name":"parse","kind":"method","module":"email","parent":"EmailAddress","url":"email.html#method-EmailAddress-parse","desc":"Parses an RFC 5322 mailbox string into an `EmailAddress` structure.  Accepts `\"Name <email@domain.co"},
  {"name":"format","kind":"method","module":"email","parent":"EmailAddress","url":"email.html#method-EmailAddress-format","desc":"Formats the address according to RFC 5322 mailbox formatting rules.  If `name` is present, returns `"},
  {"name":"isValid","kind":"method","module":"email","parent":"EmailAddress","url":"email.html#method-EmailAddress-isValid","desc":"Validates whether the email address format contains a valid local part and domain."},
  {"name":"cleanHeader","kind":"method","module":"email","parent":"EmailAddress","url":"email.html#method-EmailAddress-cleanHeader","desc":"Strips carriage returns (`\\r`) and newlines (`\\n`) from header strings to prevent CRLF injection."},
  {"name":"EmailAttachment","kind":"class","module":"email","parent":null,"url":"email.html#class-EmailAttachment","desc":"Represents an email attachment or embedded inline MIME body part.  Supports binary files (encoded via RFC 4648 Base64) a"},
  {"name":"fromText","kind":"method","module":"email","parent":"EmailAttachment","url":"email.html#method-EmailAttachment-fromText","desc":"Constructs an attachment from a plain text string payload."},
  {"name":"fromBase64","kind":"method","module":"email","parent":"EmailAttachment","url":"email.html#method-EmailAttachment-fromBase64","desc":"Constructs an attachment from a Base64-encoded binary payload."},
  {"name":"inline","kind":"method","module":"email","parent":"EmailAttachment","url":"email.html#method-EmailAttachment-inline","desc":"Constructs an embedded inline image attachment accessible via `<img src=\"cid:id\">`."},
  {"name":"size","kind":"method","module":"email","parent":"EmailAttachment","url":"email.html#method-EmailAttachment-size","desc":"Returns the estimated decoded byte length of the attachment content."},
  {"name":"EmailMessage","kind":"class","module":"email","parent":null,"url":"email.html#class-EmailMessage","desc":"Fluent builder and container for composing RFC 5322 email messages.  Supports multiple recipients (`to`, `cc`, `bcc`), `"},
  {"name":"create","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-create","desc":"Instantiates a new, empty `EmailMessage` builder."},
  {"name":"from","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-from","desc":"Sets the `From` sender address and optional display name."},
  {"name":"to","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-to","desc":"Adds a primary recipient (`To`) address."},
  {"name":"toRaw","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-toRaw","desc":"Adds a primary recipient (`To`) from a raw address string."},
  {"name":"cc","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-cc","desc":"Adds a carbon copy (`Cc`) recipient address."},
  {"name":"bcc","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-bcc","desc":"Adds a blind carbon copy (`Bcc`) recipient address."},
  {"name":"replyTo","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-replyTo","desc":"Sets the `Reply-To` address header."},
  {"name":"subject","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-subject","desc":"Sets the email subject line, automatically stripping CRLF to prevent injection attacks."},
  {"name":"text","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-text","desc":"Sets the plain text version of the message body."},
  {"name":"html","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-html","desc":"Sets the HTML formatted version of the message body."},
  {"name":"header","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-header","desc":"Sets a custom email header key and value."},
  {"name":"attach","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-attach","desc":"Attaches an `EmailAttachment` to the message."},
  {"name":"attachText","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-attachText","desc":"Ergonomic shortcut to attach a text file."},
  {"name":"attachBase64","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-attachBase64","desc":"Ergonomic shortcut to attach a Base64-encoded binary file."},
  {"name":"attachInline","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-attachInline","desc":"Ergonomic shortcut to attach an embedded inline image."},
  {"name":"renderHtml","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-renderHtml","desc":"Merges dynamic variables into an HTML template string (`{{key}}` syntax) and assigns it as `html`."},
  {"name":"renderText","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-renderText","desc":"Merges dynamic variables into a plain text template string (`{{key}}` syntax) and assigns it as `tex"},
  {"name":"toMime","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-toMime","desc":"Serializes this email message into complete RFC 5322 MIME wire format."},
  {"name":"interpolateTemplate","kind":"method","module":"email","parent":"EmailMessage","url":"email.html#method-EmailMessage-interpolateTemplate","desc":"Helper utility to substitute `{{key}}` placeholders with values from a map."},
  {"name":"MimeSerializer","kind":"class","module":"email","parent":null,"url":"email.html#class-MimeSerializer","desc":"Serializes structured `EmailMessage` models into RFC 5322 and MIME (RFC 2045/2046) documents.  Automatically handles `mu"},
  {"name":"serialize","kind":"method","module":"email","parent":"MimeSerializer","url":"email.html#method-MimeSerializer-serialize","desc":"Compiles an `EmailMessage` into an RFC 5322 formatted MIME string."},
  {"name":"encodeHeaderWords","kind":"method","module":"email","parent":"MimeSerializer","url":"email.html#method-MimeSerializer-encodeHeaderWords","desc":"Encodes header text containing non-ASCII or special characters using RFC 2047 `=?UTF-8?B?...?=` word"},
  {"name":"EmailParser","kind":"class","module":"email","parent":null,"url":"email.html#class-EmailParser","desc":"Inbound RFC 5322 and MIME email parser.  Decodes raw `.eml` files or inbound webhooks (from SendGrid, AWS SES, Postmark,"},
  {"name":"parse","kind":"method","module":"email","parent":"EmailParser","url":"email.html#method-EmailParser-parse","desc":"Parses an RFC 5322 / MIME text string into an `EmailMessage` object."},
  {"name":"extractBoundary","kind":"method","module":"email","parent":"EmailParser","url":"email.html#method-EmailParser-extractBoundary","desc":"Extracts the MIME boundary parameter from a `Content-Type` header string."},
  {"name":"parseMultipartBody","kind":"method","module":"email","parent":"EmailParser","url":"email.html#method-EmailParser-parseMultipartBody","desc":"Splits a multipart body by boundary and extracts child parts."},
  {"name":"parseMimePart","kind":"method","module":"email","parent":"EmailParser","url":"email.html#method-EmailParser-parseMimePart","desc":"Processes a single extracted MIME part and updates the message."},
  {"name":"decodeHeaderWords","kind":"method","module":"email","parent":"EmailParser","url":"email.html#method-EmailParser-decodeHeaderWords","desc":"Decodes an RFC 2047 `=?UTF-8?B?...?=` encoded-word string."},
  {"name":"splitLines","kind":"method","module":"email","parent":"EmailParser","url":"email.html#method-EmailParser-splitLines","desc":"Splits a string by newline delimiters (`\\n`)."},
  {"name":"verifyWebhookSignature","kind":"method","module":"email","parent":"EmailParser","url":"email.html#method-EmailParser-verifyWebhookSignature","desc":"Verifies the HMAC-SHA256 signature of an inbound webhook payload.  Protects incoming email webhook e"},
  {"name":"DkimConfig","kind":"class","module":"email","parent":null,"url":"email.html#class-DkimConfig","desc":"RFC 6376 DomainKeys Identified Mail (DKIM) Canonicalization and Verification Engine.  Implements `relaxed/relaxed` and `"},
  {"name":"of","kind":"method","module":"email","parent":"DkimConfig","url":"email.html#method-DkimConfig-of","desc":"Constructs a DKIM configuration instance."},
  {"name":"computeBodyHash","kind":"method","module":"email","parent":"DkimConfig","url":"email.html#method-DkimConfig-computeBodyHash","desc":"Computes the cryptographic RFC 6376 SHA-256 body hash (`bh=`) encoded in Base64."},
  {"name":"canonicalizeRelaxedHeader","kind":"method","module":"email","parent":"DkimConfig","url":"email.html#method-DkimConfig-canonicalizeRelaxedHeader","desc":"Canonicalizes a single header field line under relaxed rules (RFC 6376 §3.4.2).  Lowercases header n"},
  {"name":"formatHeader","kind":"method","module":"email","parent":"DkimConfig","url":"email.html#method-DkimConfig-formatHeader","desc":"Assembles an RFC 6376 formatted `DKIM-Signature` header value string."},
  {"name":"EmailSendResult","kind":"class","module":"email","parent":null,"url":"email.html#class-EmailSendResult","desc":"Result envelope returned by email transports upon delivery attempt."},
  {"name":"ok","kind":"method","module":"email","parent":"EmailSendResult","url":"email.html#method-EmailSendResult-ok","desc":"Factory for successful delivery result."},
  {"name":"fail","kind":"method","module":"email","parent":"EmailSendResult","url":"email.html#method-EmailSendResult-fail","desc":"Factory for failed delivery result."},
  {"name":"TestInboxTransport","kind":"class","module":"email","parent":null,"url":"email.html#class-TestInboxTransport","desc":"Zero-network, in-memory email test inbox for automated unit testing.  Captures sent messages in memory, providing inspec"},
  {"name":"create","kind":"method","module":"email","parent":"TestInboxTransport","url":"email.html#method-TestInboxTransport-create","desc":"Constructs an empty `TestInboxTransport`."},
  {"name":"send","kind":"method","module":"email","parent":"TestInboxTransport","url":"email.html#method-TestInboxTransport-send","desc":"Captures the email message into memory and returns success."},
  {"name":"count","kind":"method","module":"email","parent":"TestInboxTransport","url":"email.html#method-TestInboxTransport-count","desc":"Returns the number of captured messages."},
  {"name":"last","kind":"method","module":"email","parent":"TestInboxTransport","url":"email.html#method-TestInboxTransport-last","desc":"Returns the most recently captured email message."},
  {"name":"clear","kind":"method","module":"email","parent":"TestInboxTransport","url":"email.html#method-TestInboxTransport-clear","desc":"Clears all captured emails from the test inbox."},
  {"name":"containsTo","kind":"method","module":"email","parent":"TestInboxTransport","url":"email.html#method-TestInboxTransport-containsTo","desc":"Checks if any captured message was addressed to the specified email address."},
  {"name":"containsSubject","kind":"method","module":"email","parent":"TestInboxTransport","url":"email.html#method-TestInboxTransport-containsSubject","desc":"Checks if any captured message contains the specified substring in its subject."},
  {"name":"SmtpTransport","kind":"class","module":"email","parent":null,"url":"email.html#class-SmtpTransport","desc":"High-performance RFC 5321 SMTP client transport.  Connects via raw TCP sockets, negotiates `EHLO`, performs `AUTH LOGIN`"},
  {"name":"of","kind":"method","module":"email","parent":"SmtpTransport","url":"email.html#method-SmtpTransport-of","desc":"Constructs an `SmtpTransport` instance."},
  {"name":"verify","kind":"method","module":"email","parent":"SmtpTransport","url":"email.html#method-SmtpTransport-verify","desc":"Verifies connectivity and credentials against the SMTP server.  Executes handshake (`EHLO`), perform"},
  {"name":"send","kind":"method","module":"email","parent":"SmtpTransport","url":"email.html#method-SmtpTransport-send","desc":"Sends an `EmailMessage` via SMTP with automatic exponential backoff retries."},
  {"name":"sendOnce","kind":"method","module":"email","parent":"SmtpTransport","url":"email.html#method-SmtpTransport-sendOnce","desc":"Executes a single SMTP delivery transaction."},
  {"name":"ResendTransport","kind":"class","module":"email","parent":null,"url":"email.html#class-ResendTransport","desc":"Cloud REST API transport for Resend (https://resend.com).  Delivers emails natively over HTTPS with zero third-party dep"},
  {"name":"of","kind":"method","module":"email","parent":"ResendTransport","url":"email.html#method-ResendTransport-of","desc":"Constructs a `ResendTransport`."},
  {"name":"send","kind":"method","module":"email","parent":"ResendTransport","url":"email.html#method-ResendTransport-send","desc":"Dispatches an email via Resend's REST API."},
  {"name":"SendGridTransport","kind":"class","module":"email","parent":null,"url":"email.html#class-SendGridTransport","desc":"Cloud REST API transport for Twilio SendGrid v3 Mail Send API.  Delivers emails natively over HTTPS with zero third-part"},
  {"name":"of","kind":"method","module":"email","parent":"SendGridTransport","url":"email.html#method-SendGridTransport-of","desc":"Constructs a `SendGridTransport`."},
  {"name":"send","kind":"method","module":"email","parent":"SendGridTransport","url":"email.html#method-SendGridTransport-send","desc":"Dispatches an email via the SendGrid v3 Mail Send API."},
  {"name":"PostmarkTransport","kind":"class","module":"email","parent":null,"url":"email.html#class-PostmarkTransport","desc":"Cloud REST API transport for Postmark (https://postmarkapp.com).  Delivers transactional emails over HTTPS via the Postm"},
  {"name":"of","kind":"method","module":"email","parent":"PostmarkTransport","url":"email.html#method-PostmarkTransport-of","desc":"Constructs a `PostmarkTransport`."},
  {"name":"send","kind":"method","module":"email","parent":"PostmarkTransport","url":"email.html#method-PostmarkTransport-send","desc":"Dispatches an email via the Postmark REST API."},
  {"name":"MailgunTransport","kind":"class","module":"email","parent":null,"url":"email.html#class-MailgunTransport","desc":"Cloud REST API transport for Mailgun (https://mailgun.com).  Delivers high-volume emails over HTTPS via the Mailgun v3 M"},
  {"name":"of","kind":"method","module":"email","parent":"MailgunTransport","url":"email.html#method-MailgunTransport-of","desc":"Constructs a `MailgunTransport`."},
  {"name":"send","kind":"method","module":"email","parent":"MailgunTransport","url":"email.html#method-MailgunTransport-send","desc":"Dispatches an email via the Mailgun v3 Messages API."},
  {"name":"file","kind":"module","module":"file","parent":null,"url":"file.html","desc":"Aliases and high-level file utilities."},
  {"name":"File","kind":"namespace","module":"file","parent":null,"url":"file.html#ns-File","desc":"The `File` namespace provides file reading, writing, presence checks, deletion, and size inspection."},
  {"name":"exists","kind":"function","module":"file","parent":"File","url":"file.html#func-File-exists","desc":"Check whether a file exists at the specified path."},
  {"name":"readText","kind":"function","module":"file","parent":"File","url":"file.html#func-File-readText","desc":"Read file content as a UTF-8 string."},
  {"name":"writeText","kind":"function","module":"file","parent":"File","url":"file.html#func-File-writeText","desc":"Write text to a file, replacing existing content."},
  {"name":"appendText","kind":"function","module":"file","parent":"File","url":"file.html#func-File-appendText","desc":"Append text to the end of a file."},
  {"name":"delete","kind":"function","module":"file","parent":"File","url":"file.html#func-File-delete","desc":"Delete a file from disk."},
  {"name":"size","kind":"function","module":"file","parent":"File","url":"file.html#func-File-size","desc":"Get file size in bytes."},
  {"name":"http","kind":"module","module":"http","parent":null,"url":"http.html","desc":"Provides HttpRequest, HttpResponse, HttpServer, and HttpClient Supports HTTP methods: GET, POST, PUT, PATCH, DELETE, OPT"},
  {"name":"HttpRequest","kind":"class","module":"http","parent":null,"url":"http.html#class-HttpRequest","desc":"Represents an incoming or outgoing HTTP/1.1 request message.  Encapsulates the HTTP verb (method), request target URI pa"},
  {"name":"getHeader","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-getHeader","desc":"Retrieves the value of an HTTP header by name.  Returns an empty string `\"\"` if the header is not pr"},
  {"name":"setKeepAlive","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-setKeepAlive","desc":"Sets or clears the HTTP `Connection` keep-alive header."},
  {"name":"isGet","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-isGet","desc":"Returns `true` if this request method is `\"GET\"`."},
  {"name":"isPost","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-isPost","desc":"Returns `true` if this request method is `\"POST\"`."},
  {"name":"isPut","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-isPut","desc":"Returns `true` if this request method is `\"PUT\"`."},
  {"name":"isPatch","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-isPatch","desc":"Returns `true` if this request method is `\"PATCH\"`."},
  {"name":"isDelete","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-isDelete","desc":"Returns `true` if this request method is `\"DELETE\"`."},
  {"name":"isOptions","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-isOptions","desc":"Returns `true` if this request method is `\"OPTIONS\"`."},
  {"name":"isKeepAlive","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-isKeepAlive","desc":"Checks whether the connection should remain open for subsequent requests.  Inspects the `Connection`"},
  {"name":"parse","kind":"method","module":"http","parent":"HttpRequest","url":"http.html#method-HttpRequest-parse","desc":"Parses a raw wire-format HTTP request string into a structured `HttpRequest` instance.  Deconstructs"},
  {"name":"HttpResponse","kind":"class","module":"http","parent":null,"url":"http.html#class-HttpResponse","desc":"Represents an HTTP response message sent by an HTTP server or returned by a client.  ```syntra let res = HttpResponse.ok"},
  {"name":"getHeader","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-getHeader","desc":"Retrieves an HTTP response header by name."},
  {"name":"setKeepAlive","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-setKeepAlive","desc":"Sets or clears the `Connection` header on the response."},
  {"name":"toHttpString","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-toHttpString","desc":"Serializes this `HttpResponse` into standard RFC 7230 wire format.  ```syntra let rawHttp = res.toHt"},
  {"name":"ok","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-ok","desc":"Constructs an `HTTP 200 OK` response with the specified body content."},
  {"name":"created","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-created","desc":"Constructs an `HTTP 201 Created` response."},
  {"name":"noContent","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-noContent","desc":"Constructs an `HTTP 204 No Content` response with an empty body."},
  {"name":"text","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-text","desc":"Constructs an `HTTP 200 OK` response with `text/plain` MIME type."},
  {"name":"json","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-json","desc":"Constructs an `HTTP 200 OK` response with `application/json` MIME type."},
  {"name":"badRequest","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-badRequest","desc":"Constructs an `HTTP 400 Bad Request` error response."},
  {"name":"notFound","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-notFound","desc":"Constructs an `HTTP 404 Not Found` error response."},
  {"name":"methodNotAllowed","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-methodNotAllowed","desc":"Constructs an `HTTP 405 Method Not Allowed` response with the `Allow` header."},
  {"name":"internalServerError","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-internalServerError","desc":"Constructs an `HTTP 500 Internal Server Error` response."},
  {"name":"parse","kind":"method","module":"http","parent":"HttpResponse","url":"http.html#method-HttpResponse-parse","desc":"Parses raw HTTP wire response text into a structured `HttpResponse`."},
  {"name":"HttpServer","kind":"class","module":"http","parent":null,"url":"http.html#class-HttpServer","desc":"A high-performance, asynchronous HTTP server.  Listens on a local TCP port, accepts incoming client connections, parses "},
  {"name":"listen","kind":"method","module":"http","parent":"HttpServer","url":"http.html#method-HttpServer-listen","desc":"Binds the server to the specified TCP port and starts listening for connections."},
  {"name":"accept","kind":"method","module":"http","parent":"HttpServer","url":"http.html#method-HttpServer-accept","desc":"Accepts the next incoming client TCP connection.  Blocks until a client connection arrives."},
  {"name":"send","kind":"method","module":"http","parent":"HttpServer","url":"http.html#method-HttpServer-send","desc":"Sends an `HttpResponse` to a client stream without closing the connection.  Suitable for persistent "},
  {"name":"sendResponse","kind":"method","module":"http","parent":"HttpServer","url":"http.html#method-HttpServer-sendResponse","desc":"Sends an `HttpResponse` and immediately terminates the client connection."},
  {"name":"closeClient","kind":"method","module":"http","parent":"HttpServer","url":"http.html#method-HttpServer-closeClient","desc":"Closes a client stream socket."},
  {"name":"close","kind":"method","module":"http","parent":"HttpServer","url":"http.html#method-HttpServer-close","desc":"Closes the server listening socket and halts new incoming connections."},
  {"name":"serveReactor","kind":"method","module":"http","parent":"HttpServer","url":"http.html#method-HttpServer-serveReactor","desc":"Launches native multi-core HTTP reactor on the specified port."},
  {"name":"HttpClient","kind":"class","module":"http","parent":null,"url":"http.html#class-HttpClient","desc":"Synchronous HTTP client utility for making outgoing HTTP requests.  Supports all standard HTTP verbs with automatic conn"},
  {"name":"request","kind":"method","module":"http","parent":"HttpClient","url":"http.html#method-HttpClient-request","desc":"Executes a raw HTTP request against `host:port`."},
  {"name":"get","kind":"method","module":"http","parent":"HttpClient","url":"http.html#method-HttpClient-get","desc":"Performs an HTTP GET request."},
  {"name":"post","kind":"method","module":"http","parent":"HttpClient","url":"http.html#method-HttpClient-post","desc":"Performs an HTTP POST request."},
  {"name":"put","kind":"method","module":"http","parent":"HttpClient","url":"http.html#method-HttpClient-put","desc":"Performs an HTTP PUT request."},
  {"name":"patch","kind":"method","module":"http","parent":"HttpClient","url":"http.html#method-HttpClient-patch","desc":"Performs an HTTP PATCH request."},
  {"name":"delete","kind":"method","module":"http","parent":"HttpClient","url":"http.html#method-HttpClient-delete","desc":"Performs an HTTP DELETE request without body."},
  {"name":"deleteWithBody","kind":"method","module":"http","parent":"HttpClient","url":"http.html#method-HttpClient-deleteWithBody","desc":"Performs an HTTP DELETE request with payload body."},
  {"name":"options","kind":"method","module":"http","parent":"HttpClient","url":"http.html#method-HttpClient-options","desc":"Performs an HTTP OPTIONS request."},
  {"name":"io","kind":"module","module":"io","parent":null,"url":"io.html","desc":"Provides standard file and stream operations."},
  {"name":"fileExists","kind":"function","module":"io","parent":null,"url":"io.html#func-fileExists","desc":"Check whether a file or directory exists on the filesystem at the given path.  Performs a non-throwi"},
  {"name":"readFile","kind":"function","module":"io","parent":null,"url":"io.html#func-readFile","desc":"Reads the entire contents of a file into a UTF-8 string.  Opens the target file in read-only mode an"},
  {"name":"writeFile","kind":"function","module":"io","parent":null,"url":"io.html#func-writeFile","desc":"Writes string text contents to a target file on disk.  If the file already exists, it is truncated a"},
  {"name":"appendFile","kind":"function","module":"io","parent":null,"url":"io.html#func-appendFile","desc":"Appends string text contents to the end of a file on disk.  If the file does not exist, it is create"},
  {"name":"deleteFile","kind":"function","module":"io","parent":null,"url":"io.html#func-deleteFile","desc":"Deletes a file from the filesystem.  Permanently unlinks the target file path.  ```syntra if (fileEx"},
  {"name":"fileSize","kind":"function","module":"io","parent":null,"url":"io.html#func-fileSize","desc":"Retrieves the size of a file in bytes.  Returns `-1` if the file does not exist or metadata cannot b"},
  {"name":"json","kind":"module","module":"json","parent":null,"url":"json.html","desc":"Native JSON serialization and deserialization"},
  {"name":"JsonHelper","kind":"class","module":"json","parent":null,"url":"json.html#class-JsonHelper","desc":"Helper utilities for JSON serialization and parsing of Syntra objects, arrays, and key-value maps.  Built on top of the "},
  {"name":"parseMap","kind":"method","module":"json","parent":"JsonHelper","url":"json.html#method-JsonHelper-parseMap","desc":"Parses a JSON formatted string into a Syntra Map representation.  ```syntra let raw = \"{\\\"host\\\":\\\"l"},
  {"name":"stringifyMap","kind":"method","module":"json","parent":"JsonHelper","url":"json.html#method-JsonHelper-stringifyMap","desc":"Serializes a Syntra key-value map into a compact JSON string.  ```syntra let map = new Map<string, s"},
  {"name":"loader","kind":"module","module":"loader","parent":null,"url":"loader.html","desc":"Provides recursive multi-file dependency resolution, AST splicing, circular dependency prevention, and search path resol"},
  {"name":"ModuleLoader","kind":"class","module":"loader","parent":null,"url":"loader.html#class-ModuleLoader","desc":"Pure Syntra Module Loader & Dependency Resolver.  Resolves and loads multi-file Syntra packages recursively, parsing imp"},
  {"name":"addSearchPath","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-addSearchPath","desc":"Registers an additional directory search path for module resolution."},
  {"name":"isLoaded","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-isLoaded","desc":"Checks whether a given module file path has already been processed."},
  {"name":"markLoaded","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-markLoaded","desc":"Marks a module file path as loaded to guard against circular import loops."},
  {"name":"getParentDir","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-getParentDir","desc":"Computes the directory path enclosing a given file path."},
  {"name":"joinPath","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-joinPath","desc":"Joins a parent directory path and a relative child path cleanly with a forward slash."},
  {"name":"extractModuleName","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-extractModuleName","desc":"Extracts the canonical module identifier name from an import path string.  Strips trailing `.syn` ex"},
  {"name":"findModuleFile","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-findModuleFile","desc":"Locates an import file on the filesystem relative to the importing file or search paths.  Checks: 1."},
  {"name":"loadModuleRecursive","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-loadModuleRecursive","desc":"Recursively parses an imported module file, resolving nested imports and splicing declarations."},
  {"name":"loadProgram","kind":"method","module":"loader","parent":"ModuleLoader","url":"loader.html#method-ModuleLoader-loadProgram","desc":"Loads the root Syntra program and recursively resolves, parses, and splices all imported modules int"},
  {"name":"map","kind":"module","module":"map","parent":null,"url":"map.html","desc":"Key-Value dictionary data structure"},
  {"name":"MapEntry","kind":"class","module":"map","parent":null,"url":"map.html#class-MapEntry","desc":"Generic key-value pair container representing an entry in a `Map`.  Encapsulates immutable key references and associated"},
  {"name":"getKey","kind":"method","module":"map","parent":"MapEntry","url":"map.html#method-MapEntry-getKey","desc":"Retrieves the entry's key."},
  {"name":"getValue","kind":"method","module":"map","parent":"MapEntry","url":"map.html#method-MapEntry-getValue","desc":"Retrieves the entry's value."},
  {"name":"net","kind":"module","module":"net","parent":null,"url":"net.html","desc":"Provides socket-level networking primitives (TcpListener, TcpStream)"},
  {"name":"listen","kind":"function","module":"net","parent":null,"url":"net.html#func-listen","desc":"Binds a TCP listener socket to the specified local port.  Listens for incoming TCP connections on al"},
  {"name":"connect","kind":"function","module":"net","parent":null,"url":"net.html#func-connect","desc":"Establishes an active outgoing TCP stream connection to `host:port`.  Performs DNS resolution and in"},
  {"name":"serveHttpReactor","kind":"function","module":"net","parent":null,"url":"net.html#func-serveHttpReactor","desc":"Launches the native high-performance multi-core HTTP reactor.  Uses kernel event notification (`kque"},
  {"name":"syntax","kind":"module","module":"syntax","parent":null,"url":"syntax.html","desc":"Provides a zero-GC self-hosting Lexer, AST, and recursive-descent Parser utilizing Syntra Memory Domains (Scoped Arenas)"},
  {"name":"Span","kind":"class","module":"syntax","parent":null,"url":"syntax.html#class-Span","desc":"Source code location span tracking byte offsets, 1-based line, and 1-based column coordinates.  Attached to tokens, AST "},
  {"name":"toString","kind":"method","module":"syntax","parent":"Span","url":"syntax.html#method-Span-toString","desc":"Formats the span as a standard line:column position string."},
  {"name":"Token","kind":"class","module":"syntax","parent":null,"url":"syntax.html#class-Token","desc":"Lexical token produced by the Syntra Lexer.  Encapsulates token classification kind, matched text slice, and precise sou"},
  {"name":"toString","kind":"method","module":"syntax","parent":"Token","url":"syntax.html#method-Token-toString","desc":"Serializes token debugging information."},
  {"name":"AstNode","kind":"class","module":"syntax","parent":null,"url":"syntax.html#class-AstNode","desc":"Abstract Syntax Tree (AST) node allocated within an Arena memory domain.  Represents syntactical constructs (Programs, F"},
  {"name":"addChild","kind":"method","module":"syntax","parent":"AstNode","url":"syntax.html#method-AstNode-addChild","desc":"Appends a child AST node to this node's children list."},
  {"name":"childCount","kind":"method","module":"syntax","parent":"AstNode","url":"syntax.html#method-AstNode-childCount","desc":"Returns the total number of child nodes."},
  {"name":"getChild","kind":"method","module":"syntax","parent":"AstNode","url":"syntax.html#method-AstNode-getChild","desc":"Retrieves the child AST node at index `i`."},
  {"name":"hasChildren","kind":"method","module":"syntax","parent":"AstNode","url":"syntax.html#method-AstNode-hasChildren","desc":"Checks if this node contains at least one child node."},
  {"name":"printTree","kind":"method","module":"syntax","parent":"AstNode","url":"syntax.html#method-AstNode-printTree","desc":"Formats the AST sub-tree into an indented S-expression string."},
  {"name":"toSExpr","kind":"method","module":"syntax","parent":"AstNode","url":"syntax.html#method-AstNode-toSExpr","desc":"Serializes the entire AST tree into a multi-line S-expression string."},
  {"name":"Lexer","kind":"class","module":"syntax","parent":null,"url":"syntax.html#class-Lexer","desc":"High-performance Syntra Lexer.  Scans UTF-8 source text and produces a linear stream of strongly typed tokens. Handles c"},
  {"name":"isEof","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-isEof","desc":"Checks if the scanner cursor has reached or passed the end of the source."},
  {"name":"peek","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-peek","desc":"Peeks at the current character code without consuming it."},
  {"name":"peekAhead","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-peekAhead","desc":"Peeks ahead by `offset` characters from the current position."},
  {"name":"advance","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-advance","desc":"Consumes and returns the current character, advancing line and column counters."},
  {"name":"skipWhitespaceAndComments","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-skipWhitespaceAndComments","desc":"Skips whitespace characters (spaces, tabs, newlines) and single/multi-line comments."},
  {"name":"lexNumber","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-lexNumber","desc":"Scans a sequence of decimal digits into an `IntLit` token."},
  {"name":"lexString","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-lexString","desc":"Scans a quoted string literal and processes escape sequences (`\\n`, `\\t`, `\\r`, `\\\"`, `\\\\`)."},
  {"name":"lexIdentifier","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-lexIdentifier","desc":"Scans an identifier or keyword sequence into the appropriate keyword token or `Identifier`."},
  {"name":"nextToken","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-nextToken","desc":"Scans and returns the next token from the character stream."},
  {"name":"tokenize","kind":"method","module":"syntax","parent":"Lexer","url":"syntax.html#method-Lexer-tokenize","desc":"Tokenizes the entire source buffer until EOF into an array of tokens."},
  {"name":"Parser","kind":"class","module":"syntax","parent":null,"url":"syntax.html#class-Parser","desc":"Recursive-Descent Parser allocating AST nodes in an Arena Memory Domain.  Implements standard precedence climbing for ex"},
  {"name":"allocNode","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-allocNode","desc":"Allocates an AST node directly in the attached memory arena."},
  {"name":"isEof","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-isEof","desc":"Checks if the parser cursor is past the end of tokens or at the `EOF` token."},
  {"name":"peek","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-peek","desc":"Peeks at the current token without consuming it."},
  {"name":"peekKind","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-peekKind","desc":"Returns the kind tag of the current token."},
  {"name":"advance","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-advance","desc":"Consumes and returns the current token, advancing the cursor."},
  {"name":"check","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-check","desc":"Checks whether the current token matches the specified kind without consuming it."},
  {"name":"matchToken","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-matchToken","desc":"Consumes the current token if it matches `kind`."},
  {"name":"consume","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-consume","desc":"Asserts that the current token matches `kind` and consumes it, emitting an error otherwise."},
  {"name":"parseExpression","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseExpression","desc":"Parses an expression starting at the lowest precedence level (assignments)."},
  {"name":"parseAssignment","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseAssignment","desc":"Parses assignment expressions (`a = b = c`)."},
  {"name":"parseLogicalOr","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseLogicalOr","desc":"Parses logical OR expressions (`||`)."},
  {"name":"parseLogicalAnd","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseLogicalAnd","desc":"Parses logical AND expressions (`&&`)."},
  {"name":"parseEquality","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseEquality","desc":"Parses equality expressions (`==`, `!=`)."},
  {"name":"parseComparison","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseComparison","desc":"Parses relational comparisons (`<`, `<=`, `>`, `>=`)."},
  {"name":"parseTerm","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseTerm","desc":"Parses additive binary operations (`+`, `-`)."},
  {"name":"parseFactor","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseFactor","desc":"Parses multiplicative binary operations (`*`, `/`, `%`)."},
  {"name":"parseUnary","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseUnary","desc":"Parses prefix unary operations (`!`, `-`)."},
  {"name":"parsePostfix","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parsePostfix","desc":"Parses postfix operations: method/function calls `()`, field access `.`, and array indexing `[]`."},
  {"name":"parsePrimary","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parsePrimary","desc":"Parses primary atomic expressions (literals, identifiers, `this`, `new`, grouping, array literals)."},
  {"name":"parseStatement","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseStatement","desc":"Dispatches statement parsing (imports, namespaces, variable declarations, control flow, blocks, expr"},
  {"name":"parseVarDecl","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseVarDecl","desc":"Parses a variable declaration statement (`int x = 42;`)."},
  {"name":"parseIfStatement","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseIfStatement","desc":"Parses an `if-else` conditional statement."},
  {"name":"parseWhileStatement","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseWhileStatement","desc":"Parses a `while` loop statement."},
  {"name":"parseImportDecl","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseImportDecl","desc":"Parses an `import` statement declaration (`import \"std.http\";`)."},
  {"name":"parseNamespaceDecl","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseNamespaceDecl","desc":"Parses a `namespace Name { ... }` block declaration."},
  {"name":"parseReturnStatement","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseReturnStatement","desc":"Parses a `return` statement (`return expr;` or `return;`)."},
  {"name":"parseBlock","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseBlock","desc":"Parses a curly-brace delimited statement block `{ stmt1; stmt2; }`."},
  {"name":"parseFunctionDecl","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseFunctionDecl","desc":"Parses a function declaration (`func name(params): returnType { body }`)."},
  {"name":"parseClassDecl","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseClassDecl","desc":"Parses a class declaration with fields, constructor, and methods (`class Name { ... }`)."},
  {"name":"parseProgram","kind":"method","module":"syntax","parent":"Parser","url":"syntax.html#method-Parser-parseProgram","desc":"Parses a complete compilation unit into a `Program` AST root node.  Consumes tokens until EOF, parsi"},
  {"name":"system","kind":"module","module":"system","parent":null,"url":"system.html","desc":"Provides command-line arguments, environment variables, time, and exit controls."},
  {"name":"System","kind":"namespace","module":"system","parent":null,"url":"system.html#ns-System","desc":"The `System` namespace provides foundational operating system primitives, process lifecycle manageme"},
  {"name":"args","kind":"function","module":"system","parent":"System","url":"system.html#func-System-args","desc":"Retrieves the array of command-line arguments passed to the program upon invocation.  The first entr"},
  {"name":"println","kind":"function","module":"system","parent":"System","url":"system.html#func-System-println","desc":"Prints a message to standard output (`stdout`) followed by an operating system newline.  ```syntra S"},
  {"name":"print","kind":"function","module":"system","parent":"System","url":"system.html#func-System-print","desc":"Prints a message to standard output (`stdout`) without appending a trailing newline.  Useful for pro"},
  {"name":"exit","kind":"function","module":"system","parent":"System","url":"system.html#func-System-exit","desc":"Terminates the calling process immediately with the specified exit code.  Conventionally, an exit co"},
  {"name":"getEnv","kind":"function","module":"system","parent":"System","url":"system.html#func-System-getEnv","desc":"Reads an environment variable by name from the current process environment.  If the environment vari"},
  {"name":"timeMs","kind":"function","module":"system","parent":"System","url":"system.html#func-System-timeMs","desc":"Returns the current Unix epoch time in milliseconds.  Measures monotonic wall-clock elapsed time sin"},
  {"name":"exec","kind":"function","module":"system","parent":"System","url":"system.html#func-System-exec","desc":"Spawns a subshell process executing the specified command and returns its termination exit code.  Bl"},
  {"name":"web","kind":"module","module":"web","parent":null,"url":"web.html","desc":"High-performance asynchronous HTTP microframework, Radix Router, OpenAPI 3.0 specification generator, and embedded Swagg"},
  {"name":"Context","kind":"class","module":"web","parent":null,"url":"web.html#class-Context","desc":"HTTP execution context encapsulating the incoming client request and outgoing response.  `Context` provides ergonomic he"},
  {"name":"param","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-param","desc":"Retrieves the extracted value of a named URL path parameter."},
  {"name":"query","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-query","desc":"Retrieves the value of a parsed URL query parameter."},
  {"name":"header","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-header","desc":"Looks up an HTTP header on the incoming client request in a case-insensitive manner."},
  {"name":"body","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-body","desc":"Retrieves the raw UTF-8 string payload of the incoming HTTP request body."},
  {"name":"status","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-status","desc":"Sets the HTTP status code and standard status text on the outgoing response."},
  {"name":"setHeader","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-setHeader","desc":"Appends or updates a response header in the outgoing HTTP response."},
  {"name":"text","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-text","desc":"Responds with a plain text payload and sets `Content-Type: text/plain`."},
  {"name":"json","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-json","desc":"Responds with an application/json payload, automatically calculating `Content-Length`."},
  {"name":"html","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-html","desc":"Responds with an HTML payload and sets `Content-Type: text/html; charset=utf-8`."},
  {"name":"noContent","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-noContent","desc":"Commits an HTTP 204 No Content response with an empty body."},
  {"name":"badRequest","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-badRequest","desc":"Sends an HTTP 400 Bad Request error response with a descriptive text message."},
  {"name":"notFound","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-notFound","desc":"Sends an HTTP 404 Not Found error response with a descriptive text message."},
  {"name":"sse","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-sse","desc":"Configures the response headers for Server-Sent Events (SSE) streaming (`text/event-stream`)."},
  {"name":"event","kind":"method","module":"web","parent":"Context","url":"web.html#method-Context-event","desc":"Appends a structured Server-Sent Event (SSE) message chunk to the stream."},
  {"name":"WebSocket","kind":"class","module":"web","parent":null,"url":"web.html#class-WebSocket","desc":"Full-duplex RFC 6455 compliant WebSocket connection over TCP stream.  Supports server and client frame masking, unmaskin"},
  {"name":"computeAcceptKey","kind":"method","module":"web","parent":"WebSocket","url":"web.html#method-WebSocket-computeAcceptKey","desc":"Computes the RFC 6455 `Sec-WebSocket-Accept` verification key from client nonce."},
  {"name":"send","kind":"method","module":"web","parent":"WebSocket","url":"web.html#method-WebSocket-send","desc":"Transmits an unmasked text frame from server to client."},
  {"name":"sendAsClient","kind":"method","module":"web","parent":"WebSocket","url":"web.html#method-WebSocket-sendAsClient","desc":"Transmits a masked RFC 6455 text frame from client to server with 4-byte random XOR mask."},
  {"name":"read","kind":"method","module":"web","parent":"WebSocket","url":"web.html#method-WebSocket-read","desc":"Reads and decodes the next incoming WebSocket frame from the stream."},
  {"name":"readMessage","kind":"method","module":"web","parent":"WebSocket","url":"web.html#method-WebSocket-readMessage","desc":"Reads an incoming text message from the connection (alias to `read()`)."},
  {"name":"receive","kind":"method","module":"web","parent":"WebSocket","url":"web.html#method-WebSocket-receive","desc":"Receives the next message payload from the connection channel."},
  {"name":"close","kind":"method","module":"web","parent":"WebSocket","url":"web.html#method-WebSocket-close","desc":"Gracefully closes the WebSocket connection and releases underlying socket descriptors."},
  {"name":"WebSocketHub","kind":"class","module":"web","parent":null,"url":"web.html#class-WebSocketHub","desc":"Central registry and broadcast hub for active WebSocket connections.  Facilitates group message fan-out, connection life"},
  {"name":"register","kind":"method","module":"web","parent":"WebSocketHub","url":"web.html#method-WebSocketHub-register","desc":"Registers an active WebSocket connection into the hub."},
  {"name":"unregister","kind":"method","module":"web","parent":"WebSocketHub","url":"web.html#method-WebSocketHub-unregister","desc":"Unregisters and detaches a disconnected WebSocket from the hub."},
  {"name":"broadcast","kind":"method","module":"web","parent":"WebSocketHub","url":"web.html#method-WebSocketHub-broadcast","desc":"Broadcasts a text message payload to all currently open registered client connections."},
  {"name":"count","kind":"method","module":"web","parent":"WebSocketHub","url":"web.html#method-WebSocketHub-count","desc":"Returns the current count of registered active WebSocket connections."},
  {"name":"Route","kind":"class","module":"web","parent":null,"url":"web.html#class-Route","desc":"Routing entry binding an HTTP verb, path pattern, handler closure, and OpenAPI metadata.  Supports fluent builder chaini"},
  {"name":"with","kind":"method","module":"web","parent":"Route","url":"web.html#method-Route-with","desc":"Attaches a route-specific middleware interceptor to this route."},
  {"name":"tag","kind":"method","module":"web","parent":"Route","url":"web.html#method-Route-tag","desc":"Categorizes this route under a specific OpenAPI tag."},
  {"name":"summary","kind":"method","module":"web","parent":"Route","url":"web.html#method-Route-summary","desc":"Defines the short summary displayed in Swagger UI and OpenAPI documentation."},
  {"name":"description","kind":"method","module":"web","parent":"Route","url":"web.html#method-Route-description","desc":"Defines the detailed documentation description for this endpoint."},
  {"name":"request","kind":"method","module":"web","parent":"Route","url":"web.html#method-Route-request","desc":"Declares the data model schema expected in the request body for OpenAPI generation."},
  {"name":"body","kind":"method","module":"web","parent":"Route","url":"web.html#method-Route-body","desc":"Declares the request payload schema (alias to `request()`)."},
  {"name":"response","kind":"method","module":"web","parent":"Route","url":"web.html#method-Route-response","desc":"Documents a typed response status code, schema, and description for OpenAPI."},
  {"name":"responseSimple","kind":"method","module":"web","parent":"Route","url":"web.html#method-Route-responseSimple","desc":"Documents a simple response status code and description without an explicit schema body."},
  {"name":"Router","kind":"class","module":"web","parent":null,"url":"web.html#class-Router","desc":"Radix Trie based high-throughput URL pattern matcher and dispatcher.  Supports exact path matching, named wildcard param"},
  {"name":"addRoute","kind":"method","module":"web","parent":"Router","url":"web.html#method-Router-addRoute","desc":"Adds a new route entry to the router table."},
  {"name":"match","kind":"method","module":"web","parent":"Router","url":"web.html#method-Router-match","desc":"Resolves an incoming HTTP method and request path against the registered routing table.  Extracts se"},
  {"name":"matchSegments","kind":"method","module":"web","parent":"Router","url":"web.html#method-Router-matchSegments","desc":"Matches a pattern with parameterized segments against an incoming concrete URL path."},
  {"name":"OpenApiGenerator","kind":"class","module":"web","parent":null,"url":"web.html#class-OpenApiGenerator","desc":"Dynamic OpenAPI 3.0.3 specification generator from declared Syntra routes.  Automatically compiles paths, verbs, paramet"},
  {"name":"generate","kind":"method","module":"web","parent":"OpenApiGenerator","url":"web.html#method-OpenApiGenerator-generate","desc":"Compiles an OpenAPI 3.0.3 JSON document from the routes and schemas configured in `app`."},
  {"name":"SwaggerUi","kind":"class","module":"web","parent":null,"url":"web.html#class-SwaggerUi","desc":"Embedded zero-dependency Swagger UI bundle generator.  Renders an interactive single-page API playground connected to th"},
  {"name":"generate","kind":"method","module":"web","parent":"SwaggerUi","url":"web.html#method-SwaggerUi-generate","desc":"Generates standalone HTML document serving Swagger UI connected to `openApiPath`."},
  {"name":"App","kind":"class","module":"web","parent":null,"url":"web.html#class-App","desc":"High-level web application and server container.  Provides expressive route registration (`get`, `post`, `put`, `delete`"},
  {"name":"use","kind":"method","module":"web","parent":"App","url":"web.html#method-App-use","desc":"Registers a global middleware handler executed for all incoming HTTP requests."},
  {"name":"enableOpenApi","kind":"method","module":"web","parent":"App","url":"web.html#method-App-enableOpenApi","desc":"Enables the OpenAPI 3.0 JSON specification endpoint at the designated route path."},
  {"name":"enableSwagger","kind":"method","module":"web","parent":"App","url":"web.html#method-App-enableSwagger","desc":"Enables the embedded Swagger UI interactive API documentation playground.  Automatically enables the"},
  {"name":"get","kind":"method","module":"web","parent":"App","url":"web.html#method-App-get","desc":"Registers an HTTP GET route handler."},
  {"name":"post","kind":"method","module":"web","parent":"App","url":"web.html#method-App-post","desc":"Registers an HTTP POST route handler."},
  {"name":"put","kind":"method","module":"web","parent":"App","url":"web.html#method-App-put","desc":"Registers an HTTP PUT route handler."},
  {"name":"patch","kind":"method","module":"web","parent":"App","url":"web.html#method-App-patch","desc":"Registers an HTTP PATCH route handler."},
  {"name":"delete","kind":"method","module":"web","parent":"App","url":"web.html#method-App-delete","desc":"Registers an HTTP DELETE route handler."},
  {"name":"options","kind":"method","module":"web","parent":"App","url":"web.html#method-App-options","desc":"Registers an HTTP OPTIONS route handler."},
  {"name":"all","kind":"method","module":"web","parent":"App","url":"web.html#method-App-all","desc":"Registers a route handler matching ANY HTTP verb."},
  {"name":"ws","kind":"method","module":"web","parent":"App","url":"web.html#method-App-ws","desc":"Registers a full-duplex WebSocket connection upgrade handler at the specified path."},
  {"name":"handleWebSocketUpgrade","kind":"method","module":"web","parent":"App","url":"web.html#method-App-handleWebSocketUpgrade","desc":"Negotiates an RFC 6455 WebSocket handshake upgrade from an incoming HTTP request."},
  {"name":"handleRequest","kind":"method","module":"web","parent":"App","url":"web.html#method-App-handleRequest","desc":"Dispatches an incoming HTTP request through routing, middlewares, and returns the response."},
  {"name":"listen","kind":"method","module":"web","parent":"App","url":"web.html#method-App-listen","desc":"Binds the HTTP server to a TCP port and enters the request listener loop."}
];

document.addEventListener('DOMContentLoaded', () => {
    initTheme();
    initSearch();
    initMobileNav();
});

function initTheme() {
    const savedTheme = localStorage.getItem('syntra-doc-theme') || 
        (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
    setTheme(savedTheme);

    const toggle = document.getElementById('themeToggle');
    if (toggle) {
        toggle.addEventListener('click', () => {
            const current = document.documentElement.getAttribute('data-theme') || 'light';
            const next = current === 'dark' ? 'light' : 'dark';
            setTheme(next);
        });
    }
}

function setTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('syntra-doc-theme', theme);
    const toggle = document.getElementById('themeToggle');
    if (toggle) {
        toggle.textContent = theme === 'dark' ? '☀️' : '🌙';
    }
}

function initMobileNav() {
    const toggle = document.getElementById('mobileToggle');
    const sidebar = document.getElementById('sidebar');
    if (toggle && sidebar) {
        toggle.addEventListener('click', () => {
            sidebar.classList.toggle('open');
        });
    }
}

function initSearch() {
    const input = document.getElementById('searchInput');
    const dropdown = document.getElementById('searchResults');
    if (!input || !dropdown) return;

    let selectedIndex = -1;
    let currentResults = [];

    // Global keyboard shortcut '/' to focus search
    window.addEventListener('keydown', (e) => {
        if (e.key === '/' && document.activeElement !== input) {
            e.preventDefault();
            input.focus();
            input.select();
        }
    });

    input.addEventListener('input', () => {
        const query = input.value.trim().toLowerCase();
        if (query.length < 1) {
            dropdown.classList.add('hidden');
            dropdown.innerHTML = '';
            selectedIndex = -1;
            return;
        }

        currentResults = SEARCH_INDEX.filter(item => {
            const nameMatch = item.name.toLowerCase().includes(query);
            const parentMatch = item.parent && item.parent.toLowerCase().includes(query);
            const modMatch = item.module.toLowerCase().includes(query);
            const descMatch = item.desc && item.desc.toLowerCase().includes(query);
            return nameMatch || parentMatch || modMatch || descMatch;
        }).slice(0, 15);

        if (currentResults.length === 0) {
            dropdown.innerHTML = '<div style="padding: 1rem; color: var(--text-muted); font-size: 0.85rem; text-align: center;">No matching symbols found.</div>';
            dropdown.classList.remove('hidden');
            selectedIndex = -1;
            return;
        }

        selectedIndex = 0;
        renderSearchResults(dropdown, currentResults, selectedIndex);
        dropdown.classList.remove('hidden');
    });

    input.addEventListener('keydown', (e) => {
        if (dropdown.classList.contains('hidden') || currentResults.length === 0) return;

        if (e.key === 'ArrowDown') {
            e.preventDefault();
            selectedIndex = (selectedIndex + 1) % currentResults.length;
            highlightSelected(dropdown, selectedIndex);
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            selectedIndex = (selectedIndex - 1 + currentResults.length) % currentResults.length;
            highlightSelected(dropdown, selectedIndex);
        } else if (e.key === 'Enter') {
            e.preventDefault();
            if (selectedIndex >= 0 && selectedIndex < currentResults.length) {
                window.location.href = currentResults[selectedIndex].url;
            }
        } else if (e.key === 'Escape') {
            dropdown.classList.add('hidden');
            input.blur();
        }
    });

    // Click outside to dismiss
    document.addEventListener('click', (e) => {
        if (!e.target.closest('.search-container')) {
            dropdown.classList.add('hidden');
        }
    });
}

function renderSearchResults(dropdown, results, selectedIdx) {
    let html = '';
    results.forEach((item, idx) => {
        const isSel = idx === selectedIdx ? ' selected' : '';
        const parentDisplay = item.parent ? `${item.parent}.` : '';
        html += `
        <a href="${item.url}" class="search-item${isSel}">
            <div class="search-item-header">
                <span class="search-item-title">${parentDisplay}${item.name}</span>
                <span class="search-item-meta"><span class="kind-tag ${item.kind}">${item.kind}</span> in ${item.module}</span>
            </div>
            ${item.desc ? `<div class="search-item-desc">${item.desc}</div>` : ''}
        </a>`;
    });
    dropdown.innerHTML = html;
}

function highlightSelected(dropdown, selectedIdx) {
    const items = dropdown.querySelectorAll('.search-item');
    items.forEach((el, idx) => {
        if (idx === selectedIdx) {
            el.classList.add('selected');
            el.scrollIntoView({ block: 'nearest' });
        } else {
            el.classList.remove('selected');
        }
    });
}

function copySnippet(btn) {
    const pre = btn.closest('.code-card').querySelector('pre code');
    if (!pre) return;
    navigator.clipboard.writeText(pre.innerText).then(() => {
        const orig = btn.textContent;
        btn.textContent = '✓ Copied';
        setTimeout(() => { btn.textContent = orig; }, 1800);
    }).catch(() => {
        btn.textContent = 'Error';
    });
}
