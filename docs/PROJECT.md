# **Project Brief (Revised): Open-Core Photo Culling & Deduplication App**

## **Overview**

We are developing a **cross-platform desktop application**—built in **Rust** with a **Tauri GUI**—to help photographers and creators quickly **cull large photo sets** and **remove duplicates or similar images**. The product is positioned as an **Open-Core** application with a powerful, free, and open-source foundation and a paid "Pro" tier for advanced features. The core strategic differentiator is **on-device, privacy-first processing**.

## **Purpose**

Modern creators capture thousands of photos per session. Selecting the best shots and eliminating redundant or low-quality ones is a major operational bottleneck. Existing solutions force a compromise: either they are overly manual (Photo Mechanic), or they are cloud-bound and raise significant privacy and security concerns (Aftershoot, FilterPixel).

Our product fills this strategic gap with an offline-first, high-performance, and community-trusted tool for:

- **High-speed manual culling** (leveraging Rust for performance)
- **Visual deduplication** (exact & perceptual hashes)
- **On-device AI assists** (blur, sharpness, face detection)
- **Safe, local-first file management**

## **Key Differentiators**

- **Privacy by Design**: All photos and metadata are processed on the user's machine. **Your photos never leave your computer.** This is our core promise.
- **Open-Core Trust**: A powerful, free, and open-source core application builds community trust and drives adoption.
- **Ownership & Flexibility**: We offer both a perpetual license for users with subscription fatigue and a standard annual subscription.
- **Modern Performance**: A lean, fast, and secure application built with Rust and Tauri, ensuring a small footprint and high efficiency across macOS, Windows, and Linux.

## **Monetization Model: Open-Core**

### **Free & Open-Source Tier (The Core Product)**

The goal of the free tier is to provide immense value, build a large user base, and establish trust. It will be the best free culling tool on the market.

- **Unlimited** manual culling and folder management.
- **Best-in-class performance** for viewing, rating, and tagging.
- **Basic AI Assists**: On-device blur, sharpness, and exact duplicate detection.
- Full reporting and export capabilities (CSV/JSON).

### **Pro Tier (Paid)**

The Pro tier unlocks advanced, computationally-intensive AI features for professionals who need a competitive edge.

- **Advanced On-Device AI**:
  - Aesthetic scoring to find the most compelling images.
  - Advanced facial analysis (emotion, closed-eye detection).
  - Intelligent subject-based grouping.
- **Seamless Integrations**: Lightroom, Capture One, and cloud storage connectors.
- **Workflow Automation**: Rule-based presets and a powerful rule editor.
- **Priority Support**.

### **Pricing (Individual Pro Tier)**

| Plan Type           | Price         | Notes                                                               |
| :------------------ | :------------ | :------------------------------------------------------------------ |
| Annual Subscription | **$150/year** | Recurring subscription with continuous updates.                     |
| Perpetual License   | **$299**      | One-time payment for the current version, plus one year of updates. |

## **Revenue Projections (Open-Core Model)**

### **How These Numbers Were Estimated**

These projections are based on a strategic pivot away from generic SaaS metrics. They are grounded in an **Open-Core** model, which prioritizes building a large, engaged community with a powerful free tool and then converting a small percentage to a paid "Pro" tier.

- **Benchmarking Niche Tools**: We analyzed the growth patterns of successful open-source utilities like **Czkawka** (24k+ GitHub stars) and commercial competitors like **Aftershoot** (200k+ users) and **FilterPixel** (25k MAUs).
- **Community-Driven Growth**: Assumes organic adoption driven by word-of-mouth on platforms like GitHub, Reddit photography communities, and YouTube, rather than expensive ad campaigns. The free, high-value core product is the primary marketing engine.
- **Targeting the "Trust Gap"**: Focuses on the underserved market of professionals and prosumers who are skeptical of cloud-based AI and suffer from subscription fatigue.

### **Key Assumptions & Forecast (Year 3 Target)**

#### **Conservative Base Case**

| Metric                                  | Value       |
| :-------------------------------------- | :---------- |
| Active Free Users (MAU)                 | 15,000      |
| Core-to-Pro Conversion Rate             | 1.5%        |
| Total Paying Customers                  | 225         |
| Blended Average Revenue Per User (ARPU) | \~$165/year |

#### **Realistic Target**

| Metric                                  | Value       |
| :-------------------------------------- | :---------- |
| Active Free Users (MAU)                 | 25,000      |
| Core-to-Pro Conversion Rate             | 2.0%        |
| Total Paying Customers                  | 500         |
| Blended Average Revenue Per User (ARPU) | \~$170/year |

#### **Optimistic Scenario**

| Metric                                  | Value       |
| :-------------------------------------- | :---------- |
| Active Free Users (MAU)                 | 40,000      |
| Core-to-Pro Conversion Rate             | 2.5%        |
| Total Paying Customers                  | 1,000       |
| Blended Average Revenue Per User (ARPU) | \~$175/year |

### **Financial Projections (Year 3 Target)**

| Scenario                | Annual Revenue |
| :---------------------- | :------------- |
| **Conservative Base**   | \~$37,000      |
| **Realistic Target**    | \~$85,000      |
| **Optimistic Scenario** | \~$175,000     |

## **Go-to-Market Strategy**

Our strategy is to build a community first, then monetize it.

1. **Launch the Open-Source Core**: Release a high-quality, free, and open-source culling application on GitHub to attract early adopters and build credibility.
2. **Community Engagement**: Actively manage the community on GitHub, Discord, and Reddit. Focus on excellent documentation, tutorials, and being responsive to user feedback.
3. **Content Seeding**: Create content ("Cull 2,000 photos in 10 minutes with zero cloud uploads") that highlights the core differentiators of speed and privacy.
4. **Launch the "Pro" Tier**: Once a critical mass of free users is established, introduce the paid Pro version with advanced AI features as an upgrade path.
5. **Develop Integrations**: Build out the planned integrations for Lightroom and other professional tools to solidify the app's place in the professional workflow.

## **Summary**

This project targets a clear and growing pain point for a large and underserved segment—creators overwhelmed by photo volume who are increasingly concerned about privacy. By adopting an Open-Core strategy, we can build a sustainable business founded on trust and community.

### **Revenue Potential Summary (Year 3)**

| Scenario                | Annual Revenue |
| :---------------------- | :------------- |
| **Conservative Base**   | **\~$37,000**  |
| **Realistic Target**    | **\~$85,000**  |
| **Optimistic Scenario** | **\~$175,000** |

### **Key Outcomes**

- Serve **tens of thousands of active free users** with a best-in-class tool.
- Establish a new market category for **high-performance, privacy-first photo automation**.
- Build a sustainable, profitable business by aligning our product with the values of our users.
