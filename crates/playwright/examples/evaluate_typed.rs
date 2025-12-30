// Example: Typed evaluate() with Deserialize structs
//
// This example demonstrates the generic evaluate() method which allows you to
// specify the return type directly. Instead of getting a serde_json::Value,
// you can deserialize the result into a Rust struct that implements Deserialize.
//
// Usage: cargo run --example evaluate_typed

use playwright_rs::Playwright;
use serde::{Deserialize, Serialize};

// ============================================================================
// Define structs that represent the data you expect from JavaScript
// ============================================================================

/// A simple calculation result
#[derive(Debug, Deserialize)]
struct CalcResult {
    sum: i32,
    product: i32,
    average: f64,
}

/// User information
#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

/// User profile response
#[derive(Debug, Deserialize)]
struct UserProfile {
    user: User,
    email: String,
    is_admin: bool,
}

/// Element position on the page
#[derive(Debug, Deserialize)]
struct ElementPosition {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Launch Playwright
    println!("Launching Playwright...");
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate to a page
    page.goto("https://example.com", None).await?;

    // ========================================================================
    // Example 1: Simple numeric calculation
    // ========================================================================
    println!("\n=== Example 1: Simple Calculation ===");

    let numbers = vec![10, 20, 30];
    let calc_result: CalcResult = page
        .evaluate(
            r#"
        (numbers) => ({
            sum: numbers.reduce((a, b) => a + b, 0),
            product: numbers.reduce((a, b) => a * b, 1),
            average: numbers.reduce((a, b) => a + b, 0) / numbers.length
        })
        "#,
            Some(&numbers),
        )
        .await?;

    println!("Numbers: {:?}", numbers);
    println!("  Sum: {}", calc_result.sum);
    println!("  Product: {}", calc_result.product);
    println!("  Average: {}", calc_result.average);

    // ========================================================================
    // Example 2: Returning a structured object
    // ========================================================================
    println!("\n=== Example 2: User Profile ===");

    let user = User {
        name: "Alice".to_string(),
        age: 30,
    };

    let profile: UserProfile = page
        .evaluate(
            r#"
        (user) => ({
            user: user,
            email: `${user.name.toLowerCase()}@example.com`,
            is_admin: user.age >= 30
        })
        "#,
            Some(&user),
        )
        .await?;

    println!("User: {:?}", profile.user);
    println!("Email: {}", profile.email);
    println!("Is Admin: {}", profile.is_admin);

    // ========================================================================
    // Example 3: DOM position without arguments
    // ========================================================================
    println!("\n=== Example 3: Element Position ===");

    // Create a test element first
    let _: () = page
        .evaluate(
            r#"
        () => {
            const div = document.createElement('div');
            div.id = 'test-element';
            div.style.width = '100px';
            div.style.height = '50px';
            div.style.position = 'absolute';
            div.style.top = '10px';
            div.style.left = '20px';
            div.style.backgroundColor = 'blue';
            document.body.appendChild(div);
        }
        "#,
            None::<&()>,
        )
        .await?;

    // Now get its position
    let position: ElementPosition = page
        .evaluate(
            r#"
        () => {
            const el = document.getElementById('test-element');
            const rect = el.getBoundingClientRect();
            return {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height
            };
        }
        "#,
            None::<&()>,
        )
        .await?;

    println!("Element Position:");
    println!("  X: {}", position.x);
    println!("  Y: {}", position.y);
    println!("  Width: {}", position.width);
    println!("  Height: {}", position.height);

    // ========================================================================
    // Example 4: Basic types (no struct needed)
    // ========================================================================
    println!("\n=== Example 4: Basic Types ===");

    // Return a number
    let num: i32 = page.evaluate("() => 42", None::<&()>).await?;
    println!("Number result: {}", num);

    // Return a string
    let text: String = page
        .evaluate("() => 'Hello from JavaScript'", None::<&()>)
        .await?;
    println!("String result: {}", text);

    // Return a boolean
    let flag: bool = page.evaluate("() => true", None::<&()>).await?;
    println!("Boolean result: {}", flag);

    // ========================================================================
    // Benefits of typed evaluate()
    // ========================================================================
    println!("\n=== Benefits of Typed evaluate() ===");
    println!("1. Type Safety: Compiler checks that the return type is valid");
    println!("2. Auto-Serialization: Arguments are automatically serialized to JSON");
    println!("3. Auto-Deserialization: Results are directly converted to your structs");
    println!("4. Better Error Messages: Deserialization errors are clear");
    println!("5. IDE Support: Full autocomplete for the result object");

    browser.close().await?;
    println!("\nDone!");
    Ok(())
}
