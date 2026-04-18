from setuptools import setup, find_packages

setup(
    name="aetheris-sdk",
    version="0.1.0",
    description="Aetheris Python SDK",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    author="Aetheris Team",
    author_email="team@aetheris.dev",
    url="https://github.com/aetheris/aetheris",
    packages=find_packages(),
    install_requires=[
        "requests>=2.28.0"
    ],
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.7",
)
